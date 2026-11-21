//! System and process memory figures, used to size the 3D map's caches.
//!
//! Budgets scale with the installed RAM so the streamer behaves on a 16 GB
//! laptop as well as a 64 GB workstation. On non-Windows targets the
//! figures fall back to a conservative 16 GB machine.

#[derive(Clone, Copy, Debug)]
pub struct MemInfo {
    /// Installed physical RAM (bytes).
    pub total: u64,
    /// Physical RAM currently available to programs (bytes).
    pub avail: u64,
    /// This process's committed private memory (bytes).
    pub process: u64,
}

#[cfg(windows)]
pub fn query() -> MemInfo {
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX};
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut ms: MEMORYSTATUSEX = std::mem::zeroed();
        ms.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        let ok = GlobalMemoryStatusEx(&mut ms) != 0;
        let mut pc: PROCESS_MEMORY_COUNTERS_EX = std::mem::zeroed();
        pc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let ok2 = GetProcessMemoryInfo(GetCurrentProcess(), &mut pc as *mut _ as *mut _, pc.cb) != 0;
        MemInfo {
            total: if ok { ms.ullTotalPhys } else { 16 << 30 },
            avail: if ok { ms.ullAvailPhys } else { 8 << 30 },
            process: if ok2 { pc.PrivateUsage as u64 } else { 0 },
        }
    }
}

#[cfg(not(windows))]
pub fn query() -> MemInfo {
    MemInfo { total: 16 << 30, avail: 8 << 30, process: 0 }
}

/// Memory limits for the 3D map, derived from the installed RAM.
#[derive(Clone, Copy, Debug)]
pub struct Budget {
    /// Above this the streamer stops loading, shrinks new textures and evicts
    /// everything that is not on screen.
    pub soft: u64,
    /// Above this it also drops visible models, farthest first.
    pub hard: u64,
    /// Estimated texture bytes before new loads fall back to 64 px.
    pub textures: u64,
    /// Maximum number of resident models.
    pub models: usize,
    /// Keep at least this much RAM free for the rest of the system.
    pub keep_free: u64,
}

impl Budget {
    pub fn for_total(total: u64) -> Budget {
        const GB: u64 = 1 << 30;
        let total = total.max(4 * GB);
        let gb = total / GB;
        Budget {
            soft: (total as f64 * 0.20) as u64,
            hard: (total as f64 * 0.30) as u64,
            textures: (total / 16).clamp(GB / 2, 3 * GB),
            models: (gb as usize * 120).clamp(800, 8000),
            keep_free: (total / 8).clamp(GB, 4 * GB),
        }
    }

    /// 0 = fine, 1 = over the soft cap (or the system is low on RAM), 2 = over the hard cap.
    pub fn pressure(&self, m: &MemInfo) -> u8 {
        if m.process > self.hard || m.avail < self.keep_free / 2 {
            2
        } else if m.process > self.soft || m.avail < self.keep_free {
            1
        } else {
            0
        }
    }
}

pub fn gb(b: u64) -> f32 {
    b as f32 / (1u64 << 30) as f32
}

impl Budget {
    /// A user-chosen cap (GB) instead of the RAM-derived one; textures and
    /// model counts scale with it.
    pub fn with_cap(mut self, cap_gb: f32) -> Budget {
        let cap = (cap_gb.max(1.0) as f64 * (1u64 << 30) as f64) as u64;
        self.soft = cap;
        self.hard = (cap as f64 * 1.3) as u64;
        self.textures = self.textures.min(cap / 3);
        self.models = self.models.min((cap_gb.max(1.0) * 600.0) as usize);
        self
    }
}
