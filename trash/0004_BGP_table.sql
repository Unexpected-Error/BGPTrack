-- create table Orange (
--     id integer not null, 
--     prefixes inet[]
-- );
-- create unique index ASN on Orange (id);
-- DROP INDEX ASN;
DROP TABLE Orange;
DROP TYPE announcement;
CREATE TYPE as_path_segment
AS
(
       seq BOOL,
       confed BOOL,
       as_path bigint[]
);
CREATE TYPE announcement
AS
(
    start_time DOUBLE PRECISION,
    stop_time DOUBLE PRECISION,
    prefix  inet,
    as_path_segments as_path_segment[],
    as_path_is_seq boolean
);
CREATE TABLE Orange
(
    asn       bigint not null,
    announcements announcement[] not null
);
CREATE UNIQUE INDEX ASN on Orange (asn);