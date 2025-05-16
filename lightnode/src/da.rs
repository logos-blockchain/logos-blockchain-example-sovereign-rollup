use anyhow::Result;
use nomos_core::da::BlobId;

pub async fn sampling(_blob_id: BlobId) -> Result<()> {
    Ok(())
}
