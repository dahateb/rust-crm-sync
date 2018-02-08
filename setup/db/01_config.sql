CREATE SCHEMA config; 
CREATE SCHEMA salesforce;

CREATE TABLE config.objects (
    id SERIAL PRIMARY KEY,
    name varchar(255) null,
    db_name varchar(255) null,
    fields text,
    last_sync_time timestamp,
    created timestamp,
    updated timestamp
)