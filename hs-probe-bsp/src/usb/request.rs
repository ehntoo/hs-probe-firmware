#[derive(Copy, Clone)]
pub enum Request {
    DAP1Command(([u8; 64], usize)),
    DAP2Command(([u8; 64], usize)),
}
