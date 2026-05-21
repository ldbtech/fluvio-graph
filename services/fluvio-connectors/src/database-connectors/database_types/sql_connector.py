from dataclasses import dataclass, field
from sqlalchemy import create_engine, inspect, Engine
from typing import Any, Optional


@dataclass
class DBConfig:
    dialect:  str
    host:     str
    port:     int
    database: str
    username: str
    password: str

    @property
    def url(self) -> str:
        return (
            f"{self.dialect}://{self.username}:{self.password}"
            f"@{self.host}:{self.port}/{self.database}"
        )


# ── Schema model ──────────────────────────────────────────────────────────────

@dataclass(slots=True)
class ColumnMeta:
    name:        str
    type:        str
    nullable:    bool
    primary_key: bool          = False
    default:     Optional[Any] = None


@dataclass(slots=True)
class ForeignKeyMeta:
    constrained_columns: list[str]
    referred_table:      str
    referred_columns:    list[str]


@dataclass(slots=True)
class TableMeta:
    name:         str
    columns:      list[ColumnMeta]      = field(default_factory=list)
    foreign_keys: list[ForeignKeyMeta]  = field(default_factory=list)


@dataclass(slots=True)
class DatabaseMeta:
    tables: list[TableMeta] = field(default_factory=list)

    def get_table(self, name: str) -> Optional[TableMeta]:
        return next((t for t in self.tables if t.name == name), None)


# ── Connector ─────────────────────────────────────────────────────────────────

class DatabaseConnector:
    def __init__(self, config: DBConfig):
        self.config = config
        self.engine = create_engine(config.url)

    def get_engine(self) -> Engine:
        return self.engine


# ── Schema extraction ─────────────────────────────────────────────────────────

class SchemaFunctions:
    def __init__(self, engine: Engine):
        self.engine    = engine
        self.inspector = inspect(engine)

    def extract(self) -> DatabaseMeta:
        database = DatabaseMeta()

        for table_name in self.inspector.get_table_names():
            table_meta = TableMeta(name=table_name)

            # Primary keys
            pk_info    = self.inspector.get_pk_constraint(table_name)
            pk_columns = set(pk_info.get('constrained_columns', []))

            # Columns
            for col in self.inspector.get_columns(table_name):
                table_meta.columns.append(ColumnMeta(
                    name=        col['name'],
                    type=        str(col['type']),
                    nullable=    col['nullable'],
                    default=     col.get('default'),
                    primary_key= col['name'] in pk_columns,
                ))

            # Foreign keys
            for fk in self.inspector.get_foreign_keys(table_name):
                table_meta.foreign_keys.append(ForeignKeyMeta(
                    constrained_columns= fk['constrained_columns'],
                    referred_table=      fk['referred_table'],
                    referred_columns=    fk['referred_columns'],
                ))

            database.tables.append(table_meta)

        return database

""" Testing Code -------------------------------------------------------------s
config = DBConfig(
    dialect=  "postgresql",
    host=     "localhost",
    port=     5432,
    database= "fluvio_collab",
    username= "alidaho",
    password= "",
)

connector = DatabaseConnector(config)
schema    = SchemaFunctions(connector.get_engine())
db_meta   = schema.extract()

for table in db_meta.tables:
    print(f"\n{table.name}")
    for col in table.columns:
        pk = " PK" if col.primary_key else ""
        print(f"  {col.name}: {col.type}{pk}")
"""