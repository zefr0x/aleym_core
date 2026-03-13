use sea_orm::{
	ActiveValue::{self, Set, Unchanged},
	prelude::*,
};

use super::{
	StorageConnection, StorageError,
	entities::{prelude::*, source, source_category, source_directory, source_to_category_link},
};
use crate::{inform::Type as InformantType, net::InterfaceType as NetworkInterfaceType};

impl StorageConnection {
	/// Returns a directory identifier.
	///
	/// Root directories have no parent.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn create_source_directory(
		&self,
		parent_id: Option<Uuid>,
		name: String,
		description: Option<String>,
	) -> Result<Uuid, StorageError> {
		let directory_id = Uuid::new_v4();

		let directory = source_directory::ActiveModel {
			id: Set(directory_id),
			parent_directory: Set(parent_id),
			name: Set(name),
			description: Set(description),
		};

		tracing::debug!(directory.id = ?directory_id, "creating new source directory");

		let directory = directory.insert(&self.connection).await?;

		Ok(directory.id)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn edit_source_directory(
		&self,
		id: Uuid,
		parent_directory: ActiveValue<Option<Uuid>>,
		name: ActiveValue<String>,
		description: ActiveValue<Option<String>>,
	) -> Result<(), StorageError> {
		let directory = source_directory::ActiveModel {
			id: Unchanged(id),
			parent_directory,
			name,
			description,
		};

		tracing::debug!("editing source directory");

		directory.update(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn delete_source_directory(&self, id: Uuid) -> Result<(), StorageError> {
		tracing::trace!("deleting source directory");

		SourceDirectory::delete_by_id(id).exec(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_all_directories(&self) -> Result<Vec<source_directory::Model>, StorageError> {
		Ok(SourceDirectory::find().all(&self.connection).await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_root_directories(&self) -> Result<Vec<source_directory::Model>, StorageError> {
		Ok(SourceDirectory::find()
			.filter(source_directory::Column::ParentDirectory.is_null())
			.all(&self.connection)
			.await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_directories_by_parent(
		&self,
		parent_id: Uuid,
		recursive: bool,
	) -> Result<Vec<source_directory::Model>, StorageError> {
		use sea_orm::{
			FromQueryResult,
			sea_query::{
				ColumnRef, CommonTableExpression, Cycle, Expr, ExprTrait, JoinType, SelectStatement, TableName,
				TableRef, UnionType, WithClause,
			},
		};

		if recursive {
			let traversal = TableName::from("traversal");

			let query = SelectStatement::new()
				// Final select
				.column(ColumnRef::Asterisk(None))
				.from(traversal.clone())
				.to_owned()
				// WITH clause
				.with(
					WithClause::new()
						.recursive(true)
						.cte(
							CommonTableExpression::new()
								.query(
									SelectStatement::new()
										// Base statement
										.column(ColumnRef::Asterisk(None))
										.from(SourceDirectory)
										.and_where(Expr::col(source_directory::Column::Id).eq(parent_id))
										// Referencing
										.union(
											UnionType::All,
											SelectStatement::new()
												.column(ColumnRef::Asterisk(Some(TableName::from("d"))))
												.from(TableRef::Table(SourceDirectory.into(), Some("d".into())))
												.join(
													JoinType::InnerJoin,
													TableRef::Table(traversal.clone(), Some("r".into())),
													Expr::col(("d", source_directory::Column::ParentDirectory))
														.equals(("r", source_directory::Column::Id)),
												)
												.to_owned(),
										)
										.to_owned(),
								)
								.table_name(traversal.1)
								.to_owned(),
						)
						.cycle(Cycle::new_from_expr_set_using(
							Expr::column(source_directory::Column::Id),
							"looped",
							"traversal_path",
						))
						.to_owned(),
				);

			Ok(
				source_directory::Model::find_by_statement(self.connection.get_database_backend().build(&query))
					.all(&self.connection)
					.await?,
			)
		} else {
			Ok(SourceDirectory::find()
				.filter(source_directory::Column::ParentDirectory.eq(parent_id))
				.all(&self.connection)
				.await?)
		}
	}

	/// Returns a category identifier.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn create_source_category(
		&self,
		name: String,
		description: Option<String>,
	) -> Result<Uuid, StorageError> {
		let category_id = Uuid::new_v4();

		let category = source_category::ActiveModel {
			id: Set(category_id),
			name: Set(name),
			description: Set(description),
		};

		tracing::debug!(category.id = ?category_id, "creating new source category");

		let category = category.insert(&self.connection).await?;

		Ok(category.id)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn edit_source_category(
		&self,
		id: Uuid,
		name: ActiveValue<String>,
		description: ActiveValue<Option<String>>,
	) -> Result<(), StorageError> {
		let directory = source_category::ActiveModel {
			id: Unchanged(id),
			name,
			description,
		};

		tracing::debug!("editing source category");

		directory.update(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn delete_source_category(&self, id: Uuid) -> Result<(), StorageError> {
		tracing::trace!("deleting source category");

		SourceCategory::delete_by_id(id).exec(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_all_categories(&self) -> Result<Vec<source_category::Model>, StorageError> {
		Ok(SourceCategory::find().all(&self.connection).await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_category(&self, id: Uuid) -> Result<source_category::Model, StorageError> {
		Ok(SourceCategory::find_by_id(id)
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!(
				"Expected source_category with id = `{id}`"
			)))?)
	}

	/// Returns a source identifier.
	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn add_source(
		&self,
		parent_directory: Uuid,
		informant: InformantType,
		network: NetworkInterfaceType,
		name: String,
		description: Option<String>,
		is_enabled: bool,
	) -> Result<Uuid, StorageError> {
		let source_id = Uuid::new_v4();

		let source = source::ActiveModel {
			id: Set(source_id),
			parent_directory: Set(parent_directory),
			// TODO: Handle both informant and network parameters when implemented.
			informant: Set(informant as i8),
			network: Set(network as i8),
			name: Set(name),
			description: Set(description),
			is_enabled: Set(is_enabled),
			..Default::default()
		};

		tracing::debug!(source.id = ?source_id, "adding new source");

		let source = source.insert(&self.connection).await?;

		Ok(source.id)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn edit_source(
		&self,
		id: Uuid,
		parent_directory: ActiveValue<Uuid>,
		network: ActiveValue<NetworkInterfaceType>,
		name: ActiveValue<String>,
		description: ActiveValue<Option<String>>,
		is_enabled: ActiveValue<bool>,
	) -> Result<(), StorageError> {
		let network = match network {
			Set(network) => Set(network as i8),
			Unchanged(network) => Unchanged(network as i8),
			ActiveValue::NotSet => ActiveValue::NotSet,
		};

		let directory = source::ActiveModel {
			id: Unchanged(id),
			parent_directory,
			// TODO: Handle both network and editable informant parameters when implemented.
			network,
			name,
			description,
			is_enabled,
			..Default::default()
		};

		tracing::debug!("editing source category");

		directory.update(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn delete_source(&self, id: Uuid) -> Result<(), StorageError> {
		tracing::trace!("deleting source");

		Source::delete_by_id(id).exec(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_all_sources(&self) -> Result<Vec<source::Model>, StorageError> {
		Ok(Source::find().all(&self.connection).await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_source(&self, id: Uuid) -> Result<source::Model, StorageError> {
		Ok(Source::find_by_id(id)
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!(
				"Expected source_category with id = `{id}`"
			)))?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_sources_by_parent_directory(
		&self,
		parent_directory: Uuid,
	) -> Result<Vec<source::Model>, StorageError> {
		let parent_directory = SourceDirectory::find_by_id(parent_directory)
			// TODO: Only fetch the id and not anything else, its enough to find a relation.
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!(
				"Expected source_directory with id = `{parent_directory}`"
			)))?;

		Ok(parent_directory.find_related(Source).all(&self.connection).await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_sources_by_category(&self, category: Uuid) -> Result<Vec<source::Model>, StorageError> {
		let category = SourceCategory::find_by_id(category)
			// TODO: Only fetch the id and not anything else, its enough to find a relation.
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!(
				"Expected source_category with id = `{category}`"
			)))?;

		Ok(category.find_related(Source).all(&self.connection).await?)
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn assign_category_to_source(&self, source: Uuid, category: Uuid) -> Result<(), StorageError> {
		let category_link = source_to_category_link::ActiveModel {
			source_id: Set(source),
			category_id: Set(category),
		};

		tracing::debug!("assigning category to source");

		category_link.insert(&self.connection).await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn unassign_category_from_source(&self, source: Uuid, category: Uuid) -> Result<(), StorageError> {
		tracing::debug!("unassigning category from source");

		SourceToCategoryLink::delete_by_id((source, category))
			.exec(&self.connection)
			.await?;

		Ok(())
	}

	#[tracing::instrument(skip(self), level = tracing::Level::DEBUG)]
	pub async fn get_categories_of_source(&self, id: Uuid) -> Result<Vec<source_category::Model>, StorageError> {
		let source = Source::find_by_id(id)
			// TODO: Only fetch the id and not anything else, its enough to find a relation.
			.one(&self.connection)
			.await?
			.ok_or(DbErr::RecordNotFound(format!("Expected source with id = `{id}`")))?;

		Ok(source.find_related(SourceCategory).all(&self.connection).await?)
	}
}

#[cfg(test)]
mod tests {
	use super::StorageConnection;
	use sea_orm::{
		ActiveValue::{NotSet, Set},
		entity::prelude::Uuid,
	};
	use tracing_test::traced_test;

	pub async fn setup_direcotries(con: &StorageConnection) {
		con.create_source_directory(None, "Root 1".to_owned(), None)
			.await
			.unwrap();
		let root2 = con
			.create_source_directory(None, "Root 2".to_owned(), None)
			.await
			.unwrap();
		let example_child = con
			.create_source_directory(Some(root2), "Example Child".to_owned(), None)
			.await
			.unwrap();
		con.create_source_directory(
			Some(example_child),
			"Example Grand Child 1".to_owned(),
			Some("Example description".to_owned()),
		)
		.await
		.unwrap();
		con.create_source_directory(Some(example_child), "Example Grand Child 2".to_owned(), None)
			.await
			.unwrap();
	}

	pub async fn setup_categories(con: &StorageConnection) {
		con.create_source_category("Example 1".to_owned(), None).await.unwrap();
		con.create_source_category("Example 2".to_owned(), Some("Example description".to_owned()))
			.await
			.unwrap();
		con.create_source_category("Example 3".to_owned(), None).await.unwrap();
	}

	pub async fn setup_sources(con: &StorageConnection, directory1: Uuid, directory2: Uuid) {
		con.add_source(
			directory1,
			super::InformantType::TestPlaceholder,
			super::NetworkInterfaceType::TestPlaceholder,
			"Example 1".to_owned(),
			Some("Example description".to_owned()),
			true,
		)
		.await
		.unwrap();

		con.add_source(
			directory1,
			super::InformantType::TestPlaceholder,
			super::NetworkInterfaceType::TestPlaceholder,
			"Example 2".to_owned(),
			None,
			false,
		)
		.await
		.unwrap();

		con.add_source(
			directory2,
			super::InformantType::TestPlaceholder,
			super::NetworkInterfaceType::TestPlaceholder,
			"Example 3".to_owned(),
			None,
			true,
		)
		.await
		.unwrap();
	}

	#[tokio::test]
	#[traced_test]
	async fn source_storage_logic() {
		let con = crate::db::impl_migration::tests::test_connection_and_migrations().await;

		// Test directories

		setup_direcotries(&con).await;

		let roots = con
			.get_root_directories()
			.await
			.expect("Failed to get root directories");
		let root_directory1 = roots.get(0).expect("Faield to get first root direcotory").id;
		let root_directory2 = roots.get(1).expect("Faield to get second root direcotory").id;

		con.edit_source_directory(root_directory1, NotSet, Set("New Name".to_owned()), NotSet)
			.await
			.expect("Failed to change root source directory's name");

		assert!(
			con.get_directories_by_parent(root_directory1, false)
				.await
				.expect("Failed to get first root direcotry's childs")
				.is_empty()
		);

		let grand_childs = con
			.get_directories_by_parent(
				con.get_directories_by_parent(root_directory2, false)
					.await
					.expect("Failed to get root direcotry's childs")
					.first()
					.unwrap()
					.id,
				false,
			)
			.await
			.expect("Failed to get childs of a child direcotry");
		assert_eq!(grand_childs.len(), 2);

		assert_eq!(
			con.get_all_directories()
				.await
				.expect("Coudn't find any directory")
				.iter()
				.count(),
			5
		);

		con.delete_source_directory(root_directory1)
			.await
			.expect("Failed to delete source directory");

		assert_eq!(
			con.get_all_directories()
				.await
				.expect("There is not directory in the database")
				.iter()
				.count(),
			4
		);

		// Test categories

		setup_categories(&con).await;

		let categories = con.get_all_categories().await.expect("Coudn't find any category");

		assert_eq!(categories.len(), 3);
		con.delete_source_category(categories.first().unwrap().id)
			.await
			.unwrap();

		let categories = con.get_all_categories().await.expect("Coudn't find any category");
		assert_eq!(categories.len(), 2);

		con.edit_source_category(categories.first().unwrap().id, Set("New Name".to_owned()), NotSet)
			.await
			.expect("Failed to change category name");
		assert_eq!(
			con.get_category(categories.first().unwrap().id).await.unwrap().name,
			"New Name"
		);

		// Test sources

		setup_sources(&con, grand_childs.first().unwrap().id, grand_childs.last().unwrap().id).await;

		let sources1 = con
			.get_sources_by_parent_directory(grand_childs.first().unwrap().id)
			.await
			.unwrap();
		let sources2 = con
			.get_sources_by_parent_directory(grand_childs.last().unwrap().id)
			.await
			.unwrap();

		assert_eq!(
			&con.get_source(sources1.first().unwrap().id).await.unwrap(),
			sources1.first().unwrap()
		);

		con.edit_source(
			sources1.first().unwrap().id,
			NotSet,
			NotSet,
			Set("New Name".to_owned()),
			NotSet,
			NotSet,
		)
		.await
		.expect("Failed to change source name");

		assert_eq!(con.get_all_sources().await.unwrap().iter().count(), 3);
		con.delete_source(sources2.first().unwrap().id)
			.await
			.expect("Failed to delete source");
		assert_eq!(con.get_all_sources().await.unwrap().iter().count(), 2);

		con.assign_category_to_source(sources1.first().unwrap().id, categories.first().unwrap().id)
			.await
			.unwrap();
		con.assign_category_to_source(sources1.first().unwrap().id, categories.last().unwrap().id)
			.await
			.unwrap();
		con.assign_category_to_source(sources1.last().unwrap().id, categories.last().unwrap().id)
			.await
			.unwrap();

		{
			let categories_of_source = con
				.get_categories_of_source(sources1.first().unwrap().id)
				.await
				.unwrap()
				.iter()
				.map(|c| c.id)
				.collect::<Vec<Uuid>>();

			assert!(categories_of_source.contains(&categories.first().unwrap().id));
			assert!(categories_of_source.contains(&categories.last().unwrap().id));
		}

		con.unassign_category_from_source(sources1.first().unwrap().id, categories.first().unwrap().id)
			.await
			.unwrap();

		assert_eq!(
			con.get_categories_of_source(sources1.first().unwrap().id)
				.await
				.unwrap()
				.iter()
				.map(|c| c.id)
				.collect::<Vec<Uuid>>(),
			vec![categories.last().unwrap().id]
		);

		{
			let sources_of_category = con
				.get_sources_by_category(categories.last().unwrap().id)
				.await
				.unwrap()
				.iter()
				.map(|s| s.id)
				.collect::<Vec<Uuid>>();

			assert!(sources_of_category.contains(&sources1.first().unwrap().id));
			assert!(sources_of_category.contains(&sources1.last().unwrap().id));
		}
	}
}
