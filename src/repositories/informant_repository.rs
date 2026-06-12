use crate::models::Informant;
use async_trait::async_trait;

#[async_trait]
pub trait InformantRepository {
    async fn get_informant(&self, id: i32) -> Informant;
}

pub struct InformantRepositoryImpl {
    // db connection
}

#[async_trait]
impl InformantRepository for InformantRepositoryImpl {
    async fn get_informant(&self, id: i32) -> Informant {
        // implement db query
        unimplemented!()
    }
}