-- create table Orange (
--     id integer not null, 
--     prefixes inet[]
-- );
-- create unique index ASN on Orange (id);
DROP INDEX ASN;
DROP TABLE Orange;
CREATE TYPE announcement
AS
(
    start_time DOUBLE PRECISION,
    stop_time DOUBLE PRECISION,
    prefix  inet,
    as_path bigint[],
    as_path_is_seq boolean
);
CREATE TABLE Orange
(
    asn       bigint not null,
    announcements announcement[]
);
CREATE UNIQUE INDEX ASN on Orange (asn);