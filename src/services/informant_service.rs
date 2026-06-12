use crate::models::{Informant, NewsItem};
use crate::repositories::news_item_repository::NewsItemRepository;
use crate::repositories::informant_repository::InformantRepository;
use chrono::NaiveDateTime;

pub struct InformantService {
    news_item_repository: Box<dyn NewsItemRepository>,
    informant_repository: Box<dyn InformantRepository>,
}

impl InformantService {
    pub fn new(
        news_item_repository: Box<dyn NewsItemRepository>,
        informant_repository: Box<dyn InformantRepository>,
    ) -> Self {
        InformantService {
            news_item_repository,
            informant_repository,
        }
    }

    pub async fn fetch_news_items(
        &self,
        informant_id: i32,
        start_index: Option<String>,
        end_index: Option<String>,
        max_items: Option<i32>,
    ) -> Vec<NewsItem> {
        let informant = self.informant_repository.get_informant(informant_id).await;
        let params = informant.params;

        let mut news_items = Vec::new();

        if let Some(start_index) = start_index {
            if let Some(end_index) = end_index {
                news_items = self
                    .news_item_repository
                    .get_news_items_by_index_range(informant_id, start_index, end_index)
                    .await;
            } else {
                news_items = self
                    .news_item_repository
                    .get_news_items_by_index(informant_id, start_index)
                    .await;
            }
        } else {
            news_items = self
                .news_item_repository
                .get_news_items(informant_id, max_items)
                .await;
        }

        news_items
    }
}