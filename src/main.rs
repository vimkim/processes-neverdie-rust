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
    loop {
        sleep(Duration::from_secs(1));
        let pid = Pid::from_raw(peer_pid as i32);
        if nix::sys::signal::kill(pid, None).is_err() {
            eprintln!("[{}] peer {} died → respawning", role, peer_pid);
            let child = spawn_peer(bin, role, std::process::id());
            peer_pid = child.id();
            eprintln!("[{}] new peer has PID {}", role, peer_pid);
        }
    }
}

fn main() {
    // Skip the binary name
    let mut args = env::args().skip(1);
    // Make `role` an owned String
    let mut role = String::from("master");
    let mut peer_pid = 0u32;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--role" => {
                // take ownership of the next String
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

    // Ignore SIGCHLD so we don’t get zombies
    unsafe {
        signal(Signal::SIGCHLD, SigHandler::SigIgn).unwrap();
    }

    let me = env::args().next().unwrap();

    if role == "master" {
        let watcher = spawn_peer(&me, "watcher", std::process::id());
        println!("[master] spawned watcher PID {}", watcher.id());
        monitor_loop(watcher.id(), &me, "master");
    } else {
        println!("[watcher] watching master PID {}", peer_pid);
        monitor_loop(peer_pid, &me, "watcher");
    }
}
