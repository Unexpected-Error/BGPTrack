alter table Announcement add primary key (id);
CREATE UNIQUE INDEX ID on Announcement (id);
CREATE INDEX ASN on Announcement (asn);
CREATE INDEX WD on Announcement (withdrawal);
CREATE INDEX TS on Announcement (timestamp);