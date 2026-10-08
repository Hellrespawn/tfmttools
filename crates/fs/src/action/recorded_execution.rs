use tfmttools_core::history::PreparedAction;

use crate::error::{FsError, FsResult};
pub(super) fn recover_filesystem(_: &PreparedAction) -> FsResult<()> {
    Err(FsError::Recovery("Unsupported filesystem recovery descriptor".into()))
}
