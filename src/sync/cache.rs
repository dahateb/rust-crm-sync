use salesforce::objects::SObject;
use db::objects::ObjectConfig;

#[derive(Default)]
pub struct SyncObjectCache {
    pub sf_objects: Option<Vec<SObject>>,
    pub db_objects: Option<Vec<ObjectConfig>>
}