
pub fn sf_type_mapping(field_type: String) -> Result<&'static str, String>{
    match field_type.as_str() {
        "id" => Ok("varchar"),
        "string" => Ok("varchar"),
        "picklist" => Ok("varchar"),
        "double" => Ok("double precision"),
        "int" => Ok("integer"),
        "datetime" => Ok("timestamp"),
        "date" => Ok("timestamp"),
        "boolean" => Ok("boolean"),
        _ => return Ok("varchar")
    }
}