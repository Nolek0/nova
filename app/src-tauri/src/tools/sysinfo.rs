//! Informations système non sensibles (OS, processeur, mémoire, disque).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub cpu: String,
    pub cores: usize,
    pub ram_gb: f64,
    pub ram_used_gb: f64,
    pub hostname_hidden: bool,
}

pub fn collect() -> SystemInfo {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    sys.refresh_cpu_all();
    let cpu = sys.cpus().first().map(|c| c.brand().trim().to_string()).unwrap_or_default();
    SystemInfo {
        os: sysinfo::System::name().unwrap_or_else(|| std::env::consts::OS.into()),
        os_version: sysinfo::System::os_version().unwrap_or_default(),
        arch: std::env::consts::ARCH.into(),
        cpu,
        cores: sys.cpus().len(),
        ram_gb: sys.total_memory() as f64 / 1_073_741_824.0,
        ram_used_gb: sys.used_memory() as f64 / 1_073_741_824.0,
        hostname_hidden: true,
    }
}

pub fn ram_gb() -> u64 {
    collect().ram_gb.round() as u64
}

pub fn detect(text: &str) -> bool {
    let re = regex::Regex::new(r"(?i)(infos?|informations?|caractéristiques|config(uration)?)\s+(du\s+)?(système|systeme|de (mon|ce) (mac|pc|ordinateur))|combien de (ram|mémoire)").unwrap();
    re.is_match(text)
}

pub fn markdown(i: &SystemInfo) -> String {
    format!(
        "**Ton ordinateur**\n\n- Système : {} {}\n- Architecture : {}\n- Processeur : {} ({} cœurs)\n- Mémoire : {:.0} Go (dont {:.1} Go utilisés)\n\n<sub>Informations lues localement ; le nom de la machine et les données personnelles ne sont pas consultés.</sub>",
        i.os, i.os_version, i.arch, i.cpu, i.cores, i.ram_gb, i.ram_used_gb
    )
}
