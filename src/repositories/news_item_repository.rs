use crate::models::NewsItem;
use async_trait::async_trait;
use chrono::NaiveDateTime;

#[async_trait]
pub trait NewsItemRepository {
    async fn get_news_items(&self, informant_id: i32, max_items: Option<i32>) -> Vec<NewsItem>;
    async fn get_news_items_by_index(
        &self,
        informant_id: i32,
        index: String,
    ) -> Vec<NewsItem>;
    async fn get_news_items_by_index_range(
        &self,
        informant_id: i32,
        start_index: String,
        end_index: String,
    ) -> Vec<NewsItem>;
}

pub struct NewsItemRepositoryImpl {
    // db connection
}

#[async_trait]
impl NewsItemRepository for NewsItemRepositoryImpl {
    async fn get_news_items(&self, informant_id: i32, max_items: Option<i32>) -> Vec<NewsItem> {
        // implement db query
        unimplemented!()
    }

    async fn get_news_items_by_index(
        &self,
        informant_id: i32,
        index: String,
    ) -> Vec<NewsItem> {
        // implement db query
        unimplemented!()
    }

    async fn get_news_items_by_index_range(
        &self,
        informant_id: i32,
        start_index: String,
        end_index: String,
    ) -> Vec<NewsItem> {
        // implement db query
        unimplemented!()
    }
}