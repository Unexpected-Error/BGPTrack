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
    timestamp DOUBLE PRECISION not null,
    prefix  inet not null,
    as_path_segments as_path_segment[] not null
);
-- alter table Announcement add primary key (id);
-- CREATE INDEX ASN on Announcement (asn);
-- CREATE INDEX WD on Announcement (withdrawal);
-- CREATE INDEX TS on Announcement (timestamp);
