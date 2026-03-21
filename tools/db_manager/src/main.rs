use aleym_core::db::Migrator;
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() {
	cli::run_cli(Migrator::default()).await;
}
