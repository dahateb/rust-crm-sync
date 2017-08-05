
#[derive(Serialize, Deserialize)]
pub struct SObjectList{
    encoding: String, 
    pub sobjects: Vec<SObject>
}

#[derive(Serialize, Deserialize)]
pub struct SObject {
    label: String,
    pub createable: bool,
    pub updateable: bool,
    pub queryable: bool,
    pub layoutable: bool,
    #[serde(rename="customSetting")]
    pub custom_setting: bool,
    pub name: String
}

#[derive(Serialize, Deserialize)]
pub struct SObjectDescribe {
    label: String,
    pub createable: bool,
    pub updateable: bool,
    pub name: String,
    pub fields: Vec<Field>
}

#[derive(Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub length: u32,
    pub label: String,
    #[serde(rename="type")]
    pub sf_type: String
}