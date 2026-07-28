-- fluvio-database connects a second pool (COMPANY_DATABASE_URL) for the
-- "company brain" tables, kept in its own database rather than mixed into
-- the main fluviome schema. The default POSTGRES_DB (fluviome) is created
-- by the postgres image itself; this provisions the second one.
CREATE DATABASE fluvio_company;
