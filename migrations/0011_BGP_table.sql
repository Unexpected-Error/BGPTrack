DROP INDEX TS;
DROP INDEX WD;
DROP INDEX ASN;
DROP INDEX ID;
DROP TABLE Announcement;
DROP TYPE as_path_segment;
CREATE TYPE B
AS
(
    test1 bigint
);
CREATE TABLE A
(
    id UUID not null,
    b B
);