-- FUNCTION: salesforce.notify_change()

-- DROP FUNCTION salesforce.notify_change();

CREATE OR REPLACE FUNCTION salesforce.notify_change()
    RETURNS trigger
    LANGUAGE 'plpgsql'
AS $BODY$
    DECLARE table_lock varchar;
    BEGIN
    	SELECT  current_setting('salesforce.' || TG_TABLE_NAME ||'_lock',true) INTO table_lock;
        IF table_lock IS NULL OR table_lock <> 'lock' THEN
        	PERFORM pg_notify('salesforce_data', TG_TABLE_NAME || '_' || NEW.id);
        END IF;    
        RETURN NEW;
    END;

$BODY$;

ALTER FUNCTION salesforce.notify_change()
    OWNER TO postgres;
