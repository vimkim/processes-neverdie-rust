use nix::sys::signal::{signal, SigHandler, Signal};
use nix::unistd::Pid;
use std::env;
use std::process::{Child, Command};
use std::thread::sleep;
use std::time::Duration;

fn spawn_peer(bin: &str, role: &str, peer_pid: u32) -> Child {
    Command::new(bin)
        .arg("--role")
        .arg(role)
        .arg("--peer")
        .arg(peer_pid.to_string())
        .spawn()
        .unwrap()
}

fn monitor_loop(mut peer_pid: u32, bin: &str, role: &str) {
    // decide which process we should spawn when our peer dies
    let peer_role = if role == "master" {
        "watcher"
    } else {
        "master"
    };

    loop {
        sleep(Duration::from_secs(1));
        let pid = Pid::from_raw(peer_pid as i32);
        if nix::sys::signal::kill(pid, None).is_err() {
            eprintln!(
                "[{}] peer {} died → respawning {}",
                role, peer_pid, peer_role
            );
            let child = spawn_peer(bin, peer_role, std::process::id());
            peer_pid = child.id();
            eprintln!("[{}] new peer has PID {}", role, peer_pid);
        }
    }
}

fn main() {
    // Skip the binary name
    let mut args = env::args().skip(1);
    let mut role = String::from("master");
    let mut peer_pid = 0u32;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--role" => {
                if let Some(r) = args.next() {
                    role = r;
                }
            }
            "--peer" => {
                if let Some(p) = args.next() {
                    peer_pid = p.parse().expect("invalid peer PID");
                }
            }
            _ => {}
        }
    }

    // Ignore SIGCHLD so ended children don't become zombies
    unsafe {
        signal(Signal::SIGCHLD, SigHandler::SigIgn).unwrap();
    }

    // path to our own binary
    let me = env::args().next().unwrap();

    if role == "master" {
        // master always boots a watcher
        let watcher = spawn_peer(&me, "watcher", std::process::id());
        println!("[master] spawned watcher PID {}", watcher.id());
        monitor_loop(watcher.id(), &me, "master");
    } else {
        // watcher just watches the given master PID
        println!("[watcher] watching master PID {}", peer_pid);
        monitor_loop(peer_pid, &me, "watcher");
    }
}
