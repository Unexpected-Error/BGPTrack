DROP TABLE A;
DROP TYPE B;
CREATE TYPE B
AS
(
    test1 bigint
);
CREATE TABLE A
(
    id UUID not null,
    b B[]
);