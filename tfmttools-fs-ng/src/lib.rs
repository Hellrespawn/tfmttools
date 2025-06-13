mod action;
mod checksum;
mod transaction;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FsOption {
    DryRun,
}
