use color_eyre::Result;
use tfmttools_history_sql::Connection;

pub fn main() -> Result<()> {
    let connection = Connection::open("./test.db".into())?;

    Ok(())
}
