DROP INDEX TS;
DROP INDEX WD;
DROP INDEX ASN;
CREATE INDEX ASN on Announcement (asn);
CREATE INDEX WD on Announcement (withdrawal);
CREATE INDEX TS on Announcement (timestamp);