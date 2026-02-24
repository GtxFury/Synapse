use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "flymouse", version, about = "跨平台鼠标键盘剪贴板共享工具")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 以服务端模式运行（主控端）
    Server {
        /// 监听地址
        #[arg(short, long, default_value = "0.0.0.0:24800")]
        bind: String,
    },
    /// 以客户端模式运行（被控端）
    Client {
        /// 服务端地址
        #[arg(short, long)]
        server: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("flymouse=info".parse()?))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Server { bind } => {
            tracing::info!(addr = %bind, "starting flymouse server");
            let server = flymouse_net::Server::new(bind);
            server.run().await?;
        }
        Command::Client { server } => {
            tracing::info!(addr = %server, "connecting to flymouse server");
            let client = flymouse_net::Client::new(server);
            client.connect().await?;
        }
    }

    Ok(())
}
