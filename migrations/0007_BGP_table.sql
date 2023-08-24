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
CREATE TYPE as_path_seg_w
AS
(
    a_p_s as_path_segment[]
);
CREATE TYPE announcement
AS
(
    start_time DOUBLE PRECISION,
    stop_time DOUBLE PRECISION,
    prefix  inet,
    as_path_segments as_path_seg_w
);
CREATE TABLE Orange
(
    asn       bigint not null,
    announcements announcement[] not null
);
CREATE UNIQUE INDEX ASN on Orange (asn);