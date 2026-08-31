// services/nilkeyd/src/fscrypt.rs — Linux fscrypt v2 raw bindings
pub const FS_KEY_DESCRIPTOR_SIZE: usize = 8;
pub const FS_KEY_IDENTIFIER_SIZE: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FscryptAddKeyArg {
    pub key_spec: [u8; 8],
    pub raw_size: u32,
    pub key_id: u32,
    pub reserved: [u32; 8],
}
