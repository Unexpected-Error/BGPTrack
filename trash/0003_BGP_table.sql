-- Create a temporary table to hold the data with non-null announcements
CREATE TABLE Orange_temp AS
SELECT * FROM Orange;

-- Drop the original table
DROP TABLE Orange;

-- Recreate the Orange table with non-null announcements
CREATE TABLE Orange
(
    asn       bigint not null,
    announcements announcement[] not null
);

-- Copy data from the temporary table to the new table
INSERT INTO Orange (asn, announcements)
SELECT asn, announcements FROM Orange_temp;

-- Drop the temporary table
DROP TABLE Orange_temp;