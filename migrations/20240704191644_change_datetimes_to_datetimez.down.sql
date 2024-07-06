-- Add down migration script here
alter table trades
alter column entered_at
set data type timestamp
using entered_at at time zone 'utc';

