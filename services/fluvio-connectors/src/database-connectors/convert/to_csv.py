import csv
import io
import json
from dataclasses import dataclass

@dataclass
class CSVResult:
    filename: str
    content: bytes
    row_count: int
    columns: list[str]

def to_csv(
    table:   str,
    rows:    list[dict],
    columns: list[str],
) -> CSVResult:
    """
        Convert a list of rows dicts to a CSV File.

        Args:
            table: The name of the table to convert.
            rows: The list of rows to convert.
            columns: The list of columns to convert.
        Return:
            CSVResult with filename, bytes, row count, columns
    """
    buffer = io.StringIO()
    writer = csv.DictWriter(
        buffer,
        fieldnames = columns,
        extrasaction='ignore', # ignore columns not in the filename.
        lineterminator='\n'
    )

    writer.writeheader()

    for row in rows: 
        safe_row = {
            k: _safe_value(v)
            for k, v in row.items()
            if k in columns
        }

        writer.writerow(safe_row)
    
    content = buffer.getvalue().encode('utf-8')

    return CSVResult(
        filename = f"{table}.csv",
        content = content,
        row_count = len(rows),
        columns = columns,
    )

def _safe_value(v) -> str:
    """ Convert any python value to a CSV-safe string """
    if v is None: 
        return ''
    
    if isinstance(v, bool):
        return str(v).lower()
    if isinstance(v, (dict, dict)):
        return json.dumps(v)
    return str(v)

if __name__ == "__main__":
        # Simulate what DB rows look like
    rows = [
        {"id": "abc-123", "name": "Alice", "email": "alice@co.com", "role": "owner"},
        {"id": "def-456", "name": "Bob",   "email": None,           "role": "contributor"},
    ]
    columns = ["id", "name", "email", "role"]

    result = to_csv("users", rows, columns)
    print(f"Filename:  {result.filename}")
    print(f"Rows:      {result.row_count}")
    print(f"Columns:   {result.columns}")
    print(f"\nContent:\n{result.content.decode()}")