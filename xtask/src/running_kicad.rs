//! KiCad's PCB editor running with its IPC API, for the end-to-end test of
//! routing the board open in KiCad: KiCad 9 and 10 serve the API only from the
//! GUI, so the editor runs from the Docker image on a virtual display.
//!
//! Linux only: the API's Unix socket is shared through a bind mount, which
//! Docker Desktop does not pass through on macOS or Windows.

use anyhow::{Context, Result, bail, ensure};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use traxide_board_access::testing;
use traxide_contracts::Design;

/// How long KiCad may take to start and load the board.
const STARTUP: Duration = Duration::from_secs(90);

/// A PCB editor with a board open; stopped when dropped.
pub struct RunningKicad {
    container: String,
    display: Child,
    /// Where KiCad created its API's socket.
    pub socket: PathBuf,
    /// The board's folder, shared with KiCad, which saves the board there.
    pub project: PathBuf,
}

impl RunningKicad {
    /// Starts KiCad from `image` on a copy of the board `board` and its project
    /// files, all placed in `folder`, and waits until it serves the board.
    pub fn open(image: &str, board: &Path, folder: &Path) -> Result<Self> {
        let config = folder.join("config/10.0");
        let socket_folder = folder.join("socket");
        let project = folder.join("project");
        if folder.exists() {
            std::fs::remove_dir_all(folder)?;
        }
        for created in [&config, &socket_folder, &project] {
            std::fs::create_dir_all(created)?;
        }
        let stem = board.file_stem().context("a board file")?;
        for extension in ["kicad_pcb", "kicad_pro", "kicad_dru"] {
            let file = board.with_extension(extension);
            if file.is_file() {
                std::fs::copy(&file, project.join(stem).with_extension(extension))?;
            }
        }
        // The image runs KiCad as its own user, who must write the
        // configuration, the socket and the saved board.
        let open_to_kicad = || run(Command::new("chmod").arg("-R").arg("a+rwX").arg(folder));
        open_to_kicad()?;
        write_configuration(image, &config)?;
        open_to_kicad()?;

        let container = format!("traxide-e2e-kicad-{}", std::process::id());
        let board_name = board.file_name().context("a board file")?.to_string_lossy();
        // xvfb-run keeps a virtual display for as long as the editor runs; -ac
        // because the container's host name does not match its cookie, and the
        // throwaway display has nothing to protect.
        let display = Command::new("xvfb-run")
            .args(["-a", "-s", "-ac -screen 0 1600x1000x24"])
            .args([
                "docker", "run", "--rm", "--name", &container, "-e", "DISPLAY",
            ])
            .args(["-v", "/tmp/.X11-unix:/tmp/.X11-unix"])
            .arg("-v")
            .arg(format!(
                "{}:/home/kicad/.config/kicad",
                folder.join("config").display()
            ))
            .arg("-v")
            .arg(format!("{}:/tmp/kicad", socket_folder.display()))
            .arg("-v")
            .arg(format!("{}:/project", project.display()))
            .args([image, "pcbnew", &format!("/project/{board_name}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("starting KiCad on a virtual display: are xvfb-run and docker installed?")?;
        let kicad = Self {
            container,
            display,
            socket: socket_folder.join("api.sock"),
            project,
        };
        kicad.wait_for_the_board()?;
        Ok(kicad)
    }

    /// KiCad creates its socket before it has loaded the board, and answers
    /// that no board is open until then.
    fn wait_for_the_board(&self) -> Result<()> {
        let started = Instant::now();
        loop {
            match self.design() {
                Ok(_) => return Ok(()),
                Err(error) if started.elapsed() > STARTUP => {
                    bail!(
                        "KiCad did not serve the board within {} s: {error:#}",
                        STARTUP.as_secs()
                    )
                }
                Err(_) => std::thread::sleep(Duration::from_millis(500)),
            }
        }
    }

    /// The design of the open board, read through KiCad's API.
    pub fn design(&self) -> Result<Design> {
        Ok(testing::load_design_in_kicad(&self.socket, &self.project)?)
    }

    pub fn save(&self) -> Result<()> {
        Ok(testing::save_board_in_kicad(&self.socket)?)
    }

    pub fn undo(&self) -> Result<()> {
        Ok(testing::undo_in_kicad(&self.socket)?)
    }

    /// `traxide` with the environment KiCad gives a plugin it starts.
    pub fn plugin_command(&self, traxide: &Path) -> Command {
        let mut command = Command::new(traxide);
        command
            .env(
                "KICAD_API_SOCKET",
                format!("ipc://{}", self.socket.display()),
            )
            .env("KIPRJMOD", &self.project);
        command
    }
}

impl Drop for RunningKicad {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.container])
            .output();
        let _ = self.display.wait();
    }
}

/// KiCad greets a new user with a wizard, a modal dialog that keeps the API
/// busy until someone closes it, unless all three global library tables exist
/// and the wizard's two questions count as answered.
fn write_configuration(image: &str, config: &Path) -> Result<()> {
    run(Command::new("docker")
        .args(["run", "--rm", "-v"])
        .arg(format!("{}:/out", config.display()))
        .args([
            image,
            "sh",
            "-c",
            "cp ~/.config/kicad/10.0/fp-lib-table ~/.config/kicad/10.0/sym-lib-table /out/",
        ]))?;
    std::fs::write(
        config.join("design-block-lib-table"),
        "(design_block_lib_table\n\t(version 7)\n\t(lib (name \"KiCad\") (type \"Table\") \
         (uri \"/usr/share/kicad/template/design-block-lib-table\") (options \"\") \
         (descr \"KiCad Default Libraries\"))\n)\n",
    )?;
    std::fs::write(
        config.join("kicad_common.json"),
        r#"{
  "api": { "enable_server": true },
  "do_not_show_again": { "data_collection_prompt": true, "update_check_prompt": true }
}
"#,
    )?;
    Ok(())
}

fn run(command: &mut Command) -> Result<()> {
    let output = command
        .output()
        .with_context(|| format!("running {command:?}"))?;
    ensure!(
        output.status.success(),
        "{command:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
