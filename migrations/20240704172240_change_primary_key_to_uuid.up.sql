-- Add up migration script here

-- add a new uuid column with a default value of uuid_generate_v4()
alter table trades
add column new_id uuid default uuid_generate_v4();

-- populate the new uuid column with uuids
update trades
set new_id = uuid_generate_v4();

-- drop the old primary key constraint
alter table trades
drop constraint trades_pkey;

-- set the new uuid column as the primary key
alter table trades
add constraint trades_pkey primary key (new_id);

-- drop the old serial id column 
alter table trades
drop column id;

-- rename the new uuid column to id
alter table trades
rename column new_id to id;

