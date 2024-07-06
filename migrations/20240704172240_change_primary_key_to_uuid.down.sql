-- Add down migration script here

-- add a new serial column
alter table trades
add column new_serial_id serial;

-- create a sequence for the new serial ids
create sequence if not exists temp_serial_seq;

-- populate the new serial column with incremental values
update trades
set new_serial_id = nextval('temp_serial_seq');

-- drop the uuid primary key constraint
alter table trades
drop constraint trades_pkey;

-- set the new serial column as the primary key
alter table trades
add constraint trades_pkey primary key (new_serial_id);

-- drop the uuid column (optional)
alter table trades
drop column id;

-- rename the new serial column to id
alter table trades
rename column new_serial_id to id;

-- drop the temporary sequence (optional)
drop sequence if exists temp_serial_seq;

