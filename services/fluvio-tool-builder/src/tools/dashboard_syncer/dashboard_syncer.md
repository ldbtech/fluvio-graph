# Executive Dashboard Publisher

## Purpose
Publishes BI dashboards (PowerBI / Tableau) AND generates executive PDF reports
compiled from real database analytics. This is the final reporting stage of a
data pipeline: it turns `*_analytics` tables into shareable visuals or a printed
PDF document.

## Actions

### `publish_report`
Deploys a report/workbook to a BI workspace and returns share/embed links.
Arguments:
- `context`: `{ platform: "tableau" | "powerbi", workspace_id, ...credentials }`
- `report_name`: human title for the published report
- `datasource_name`: the analytics datasource/table the report binds to

### `trigger_refresh`
Re-syncs a published dataset/workbook with the latest database rows.
Arguments: `context`, `dataset_id`.

### `get_share_link`
Returns a secure embed/sharing URL. Arguments: `context`, `report_id`.

### `generate_pdf_report`
Generates a publication-quality PDF. Compiles with `pdflatex` when available,
falling back to a ReportLab renderer for the *document* only. **You (the planning
agent) author EVERYTHING**: the full LaTeX document and the actual
seaborn/matplotlib code that draws the figures. There is no auto-generated report
and no chart-type guessing. If something fails, you are told why and you retry.
Arguments:
- `context`: `{ platform: "local_pdf", environment: "local" }`
- `report_name`: the document title
- `database_url`: the Postgres URL your chart code reads from
- `latex_content`: **required** — the complete LaTeX document you authored
- `chart_code`: a list of Python snippets (seaborn/matplotlib) that produce the PNGs

#### Charting is a skill, not a menu
You write real plotting code. Before you do, you have already (earlier in the
plan) cleaned the data and built `*_analytics` tables, and you know the company
from the **knowledge graph**. Let all of that shape the charts:
- **Knowledge graph / company context** → what metrics matter, sensible groupings,
  domain-appropriate framing, brand-ish palette and tone.
- **Your cleaning + analytics** → read the exact `clean_*` / `*_analytics` tables
  and columns you produced. Never invent columns.
- **The user's question** → choose the chart that answers it (trend → line,
  composition → stacked bar/pie, distribution → hist/box, relationship → scatter,
  two-category intensity → heatmap). You decide; the runtime never infers.

#### Execution environment for each `chart_code` snippet
Each snippet runs with these names already bound:
- `pd`, `np`, `plt`, `sns` — pandas, numpy, matplotlib.pyplot (headless), seaborn
- `db_url` — the Postgres connection string
- `output_dir` — directory to save PNGs into (this is where LaTeX looks)
- `load_df(sql)` — returns a DataFrame for a query (reuses one connection)

Save each figure with `plt.savefig(os.path.join(output_dir, "<name>.png"), dpi=300)`
and reference it from the LaTeX as `\includegraphics{<name>.png}`. Snippets run in
order; figure state is reset between them.

Example snippet:
```python
df = load_df("SELECT country, revenue FROM revenue_by_country_analytics ORDER BY revenue DESC")
plt.figure(figsize=(7, 3.5))
sns.barplot(data=df, x="revenue", y="country", palette="crest", hue="country", legend=False)
plt.title("Revenue by Country", fontsize=11, fontweight="bold", pad=15)
plt.tight_layout()
plt.savefig(os.path.join(output_dir, "revenue_by_country.png"), dpi=300)
```

#### Flow
Provide `latex_content` (full `\documentclass … \end{document}`, real narrative and
tables tied to the user's question) AND a `chart_code` list whose saved filenames
match your `\includegraphics` directives. The runtime runs the chart code first so
every figure exists, then compiles the document. A missing `latex_content` or a
failing snippet returns `status: "failed"` — fix and retry.

## Common Patterns
### Metric Reporting Loop
`data-cleaning → spark (→ *_analytics tables) → dashboard-syncer.trigger_refresh → live visuals`

### Executive PDF Pipeline
`data-cleaning → spark (→ *_analytics tables) → dashboard-syncer.generate_pdf_report`
The PDF step must run AFTER the `*_analytics` tables exist, so the charts have data.

## Constraints
- Real cloud publishing needs valid API tokens; without them it falls back to a
  realistic local simulation link.
- For `generate_pdf_report`, your `chart_code` reads from the tables you produced
  upstream — ensure the cleaning + spark/analytics steps ran first so the data
  exists. `latex_content` is required; there is no auto-generated report.
