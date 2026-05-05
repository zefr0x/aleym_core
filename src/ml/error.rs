#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum SchedulerError {
	#[error("Source with id = `{0}` is not scheduled")]
	SourceNotScheduled(uuid::Uuid),
}
