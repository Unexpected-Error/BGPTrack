create table Orange (
    id integer not null,
    prefixes inet[]
);
create unique index ASN on Orange (id);