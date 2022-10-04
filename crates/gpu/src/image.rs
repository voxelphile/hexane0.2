use bitflags::bitflags;

bitflags! {
    pub struct ImageUsage: u32 {
        const TRANSFER_SRC = 0x00000001;
        const TRANSFER_DST = 0x00000002;
        const SAMPLED = 0x00000004;
        const STORAGE = 0x00000008;
        const COLOR = 0x00000010;
        const DEPTH_STENCIL = 0x00000020;
        const TRANSIENT = 0x00000040;
        const INPUT = 0x00000080;
    }
}
