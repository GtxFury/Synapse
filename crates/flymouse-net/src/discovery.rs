use anyhow::Result;
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use tracing::info;

const SERVICE_TYPE: &str = "_flymouse._tcp.local.";

/// mDNS 设备发现
pub struct Discovery {
    daemon: ServiceDaemon,
}

impl Discovery {
    pub fn new() -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        Ok(Self { daemon })
    }

    /// 注册本机服务
    pub fn register(&self, name: &str, port: u16) -> Result<()> {
        let host = format!("{}.local.", hostname::get()?.to_string_lossy());
        let service = ServiceInfo::new(SERVICE_TYPE, name, &host, "", port, None)?;
        self.daemon.register(service)?;
        info!(name, port, "registered mDNS service");
        Ok(())
    }

    /// 浏览网络中的其他设备
    pub fn browse(&self) -> Result<mdns_sd::Receiver<ServiceEvent>> {
        let receiver = self.daemon.browse(SERVICE_TYPE)?;
        info!("browsing for flymouse devices");
        Ok(receiver)
    }

    /// 关闭 mDNS 守护进程
    pub fn shutdown(self) -> Result<()> {
        self.daemon.shutdown()?;
        Ok(())
    }
}
