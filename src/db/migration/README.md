# Database Migrations Edit Policy

It's acceptable to modify database migrations during development (between commits or in `alpha` releases). Once a
migration has been shipped in a `beta`, `release-candidate (rc)`, or `stable` release, it is frozen and must not be
altered, instead, any further schema or data changes must be implemented in a new migration.
