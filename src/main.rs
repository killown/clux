mod config;
mod grabs;
mod handlers;
mod input;
mod state;
mod udev;
mod winit;

pub use state::Clux;

static POSSIBLE_BACKENDS: &[&str] = &[
    "--winit : Run clux as a X11 or Wayland client using winit.",
    "--tty-udev : Run clux on a tty using udev.",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let arg = std::env::args().nth(1);
    match arg.as_deref() {
        Some("--winit") => {
            tracing::info!("Starting clux with winit backend");
            crate::winit::run_winit()?;
        }
        Some("--tty-udev") => {
            tracing::info!("Starting clux on a tty using udev");
            crate::udev::run_udev()?;
        }
        _ => {
            println!("USAGE: clux --backend");
            println!();
            println!("Possible backends are:");
            for b in POSSIBLE_BACKENDS {
                println!("\t{b}");
            }
        }
    }

    Ok(())
}

fn init_logging() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("clux=info,smithay=info"));

    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}
