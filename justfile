_default:
	@just --list

lint_all:
	cargo clippy
	pre-commit run --all-files

todo:
	rg --hidden --glob !.git --glob !{{ file_name(justfile()) }} "( (TODO|FIX|HACK|WARN|PREF): )|(todo!)|(unimplemented!)"

clean:
	cargo clean

db-generate-entities:
	# Generate entities code from database
	cargo run -p db_manager -- fresh --database-url='sqlite://{{justfile_directory()}}/target/db.sqlite?mode=rwc'
	sea-orm-cli generate entity --date-time-crate=time --database-url='sqlite://{{justfile_directory()}}/target/db.sqlite?mode=rwc' --output-dir={{justfile_directory()}}/src/db/entities/

db-new-migration name:
	# Generate code for a new, named migration
	sea-orm-cli migrate generate --migration-dir={{justfile_directory()}}/src/db/migration {{name}}
