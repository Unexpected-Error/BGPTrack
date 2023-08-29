DROP INDEX ASN;
DROP TABLE Orange;
DROP TYPE announcement;
DROP TYPE as_path_segment;

CREATE TYPE as_path_segment
AS
(
    seq boolean,
    confed boolean,
    as_path bigint[]
);
CREATE TABLE Announcement
(
    id UUID not null,
    asn bigint not null,
    withdrawal boolean not null,
    timestamp DOUBLE PRECISION,
    prefix  inet not null,
    as_path_segments as_path_segment[]
);
CREATE UNIQUE INDEX ASN on Announcement (asn);
CREATE UNIQUE INDEX ID on Announcement (id);
CREATE UNIQUE INDEX WD on Announcement (withdrawal);
CREATE UNIQUE INDEX TS on Announcement (timestamp);