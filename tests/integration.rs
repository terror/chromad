#![cfg(unix)]

use {
  anyhow::{Error, bail},
  executable_path::executable_path,
  pretty_assertions::assert_eq,
  std::{
    fs,
    iter::once,
    net::{SocketAddr, TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    process::{Child, Command, Stdio},
    str, thread,
    time::Duration,
  },
  tempfile::TempDir,
};

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
struct Daemon {
  address: SocketAddr,
  child: Child,
  tempdir: TempDir,
}

#[derive(Debug)]
struct Test {
  arguments: Vec<String>,
  daemon: Daemon,
  expected_status: i32,
  expected_stderr: String,
  expected_stdout: String,
}

impl Daemon {
  fn command(&self, arguments: &[&str]) -> Command {
    let mut command = Command::new(executable_path(env!("CARGO_PKG_NAME")));

    command
      .arg("--address")
      .arg(self.address.to_string())
      .args(arguments)
      .env("NO_COLOR", "1")
      .env("RUST_BACKTRACE", "0");

    command
  }

  fn new() -> Result<Self> {
    let tempdir = TempDir::with_prefix("chromad-test")?;
    let chromium = tempdir.path().join("chromium");
    fs::write(
      &chromium,
      r#"#!/bin/sh
profile=
for argument in "$@"; do
  case "$argument" in
    --user-data-dir=*) profile=${argument#--user-data-dir=} ;;
  esac
done
mkdir -p "$profile"
printf 'profile state\n' > "$profile/state"
printf '9\n/devtools/browser/test\n' > "$profile/DevToolsActivePort"
exec sleep 3600
"#,
    )?;
    fs::set_permissions(&chromium, fs::Permissions::from_mode(0o755))?;

    let listener = TcpListener::bind("127.0.0.1:0")?;
    let address = listener.local_addr()?;
    drop(listener);

    let child = Command::new(executable_path(env!("CARGO_PKG_NAME")))
      .arg("--address")
      .arg(address.to_string())
      .arg("serve")
      .arg("--chromium")
      .arg(chromium)
      .arg("--data-dir")
      .arg(tempdir.path().join("data"))
      .env("RUST_BACKTRACE", "0")
      .stdin(Stdio::null())
      .stdout(Stdio::null())
      .stderr(Stdio::null())
      .spawn()?;

    let mut daemon = Self {
      address,
      child,
      tempdir,
    };
    daemon.wait_until_ready()?;
    Ok(daemon)
  }

  fn normalize(&self, text: &str) -> String {
    text
      .replace(&self.tempdir.path().display().to_string(), "[ROOT]")
      .replace('\\', "/")
  }

  fn wait_until_ready(&mut self) -> Result {
    for _ in 0..100 {
      if let Some(status) = self.child.try_wait()? {
        bail!("daemon exited before becoming ready: {status}");
      }

      if TcpStream::connect(self.address).is_ok() {
        return Ok(());
      }

      thread::sleep(Duration::from_millis(20));
    }

    bail!("daemon did not become ready")
  }
}

impl Drop for Daemon {
  fn drop(&mut self) {
    let _ = Command::new("kill")
      .arg("-INT")
      .arg(self.child.id().to_string())
      .status();

    for _ in 0..100 {
      if self.child.try_wait().is_ok_and(|status| status.is_some()) {
        return;
      }
      thread::sleep(Duration::from_millis(10));
    }

    let _ = self.child.kill();
    let _ = self.child.wait();
  }
}

impl Test {
  fn argument(self, argument: &str) -> Self {
    Self {
      arguments: self
        .arguments
        .into_iter()
        .chain(once(argument.to_owned()))
        .collect(),
      ..self
    }
  }

  fn expected_status(self, expected_status: i32) -> Self {
    Self {
      expected_status,
      ..self
    }
  }

  fn expected_stderr(self, expected_stderr: &str) -> Self {
    Self {
      expected_stderr: expected_stderr.to_owned(),
      ..self
    }
  }

  fn expected_stdout(self, expected_stdout: &str) -> Self {
    Self {
      expected_stdout: expected_stdout.to_owned(),
      ..self
    }
  }

  fn new() -> Result<Self> {
    Ok(Self {
      arguments: Vec::new(),
      daemon: Daemon::new()?,
      expected_status: 0,
      expected_stderr: String::new(),
      expected_stdout: String::new(),
    })
  }

  fn run(self) -> Result {
    let arguments = self
      .arguments
      .iter()
      .map(String::as_str)
      .collect::<Vec<_>>();
    let output = self.daemon.command(&arguments).output()?;
    let stderr = self.daemon.normalize(str::from_utf8(&output.stderr)?);
    let stdout = self.daemon.normalize(str::from_utf8(&output.stdout)?);

    assert_eq!(
      output.status.code(),
      Some(self.expected_status),
      "unexpected exit status\nstderr: {stderr}"
    );
    assert_eq!(stderr, self.expected_stderr);
    assert_eq!(stdout, self.expected_stdout);
    Ok(())
  }
}

#[test]
fn create_rejects_invalid_session_name() -> Result {
  Test::new()?
    .argument("create")
    .argument("--")
    .argument("-github")
    .expected_status(1)
    .expected_stderr(
      "error: failed to create session\n\nbecause:\n- session IDs must be 1-64 characters, start with a letter or number, and contain only ASCII letters, numbers, '-', '_', or '.'\n",
    )
    .run()
}

#[test]
fn list_is_empty_initially() -> Result {
  Test::new()?.argument("list").expected_stdout("").run()
}

#[test]
fn session_lifecycle_preserves_profile() -> Result {
  let daemon = Daemon::new()?;

  let create = daemon.command(&["create", "github"]).output()?;
  assert!(create.status.success());
  assert_eq!(
    str::from_utf8(&create.stdout)?,
    format!("ws://{}/session/github\n", daemon.address)
  );

  let list = daemon.command(&["list"]).output()?;
  assert!(list.status.success());
  assert_eq!(
    str::from_utf8(&list.stdout)?,
    format!(
      "NAME\tCDP ENDPOINT\ngithub\tws://{}/session/github\n",
      daemon.address
    )
  );

  let inspect = daemon.command(&["inspect", "github"]).output()?;
  assert!(inspect.status.success());
  let inspect = daemon.normalize(str::from_utf8(&inspect.stdout)?);
  assert!(inspect.starts_with(&format!(
    "Name: github\nCDP endpoint: ws://{}/session/github\nProfile: [ROOT]/data/sessions/github/profile\nPID: ",
    daemon.address
  )));

  let kill = daemon.command(&["kill", "github"]).output()?;
  assert!(kill.status.success());
  assert_eq!(kill.stdout, b"");
  assert!(
    daemon
      .tempdir
      .path()
      .join("data/sessions/github/profile/state")
      .is_file()
  );

  let list = daemon.command(&["list"]).output()?;
  assert!(list.status.success());
  assert_eq!(list.stdout, b"");

  Ok(())
}
