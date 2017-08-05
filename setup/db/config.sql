CREATE SCHEMA config; 

CREATE TABLE config.objects (
    id SERIAL PRIMARY KEY,
    name varchar(255) null,
    fields text,
    created timestamp,
    updated timestamp
)