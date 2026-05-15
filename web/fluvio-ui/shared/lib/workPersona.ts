/**
 * Work persona — chosen under Sources → Specialized until onboarding persists it server-side.
 * Storage key is versioned so we can migrate shapes without silent corruption.
 */
export type WorkPersonaId = "software" | "finance" | "student" | "other";

const STORAGE_KEY = "fluvio_work_persona_v1";

/** Older UI ids still in localStorage — rewritten on read. */
const LEGACY_PERSONA_IDS: Record<string, WorkPersonaId> = {
  developer: "software",
  mechanical_engineering: "other",
};

export const WORK_PERSONA_OPTIONS: { id: WorkPersonaId; label: string }[] = [
  { id: "software", label: "Software" },
  { id: "finance", label: "Finance" },
  { id: "student", label: "Student" },
  { id: "other", label: "Others" },
];

export const DEFAULT_WORK_PERSONA: WorkPersonaId = "other";

export function parseWorkPersonaId(raw: string | null | undefined): WorkPersonaId | null {
  if (!raw) return null;
  const ok = WORK_PERSONA_OPTIONS.some((o) => o.id === raw);
  return ok ? (raw as WorkPersonaId) : null;
}

export function getStoredWorkPersona(): WorkPersonaId | null {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return null;
    const normalized = LEGACY_PERSONA_IDS[raw] ?? raw;
    const parsed = parseWorkPersonaId(normalized);
    if (parsed && raw !== parsed) setStoredWorkPersona(parsed);
    return parsed;
  } catch {
    return null;
  }
}

export function setStoredWorkPersona(id: WorkPersonaId): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(STORAGE_KEY, id);
  } catch {
    /* quota / private mode */
  }
}

export type SpecializedSourcePreview = {
  title: string;
  hint: string;
  /** Short list of planned integrations (UI only). */
  planned: string[];
};

export function specializedSourcePreviewForPersona(persona: WorkPersonaId): SpecializedSourcePreview {
  switch (persona) {
    case "software":
      return {
        title: "More for software",
        hint: "After your linked repository: chat, calendar, and issue trackers will connect here next.",
        planned: ["Slack or Teams", "Calendar (meetings & focus blocks)", "Issues (Linear / Jira)"],
      };
    case "finance":
      return {
        title: "More for finance",
        hint: "Beyond Yahoo Finance & business: calendars, spreadsheets, and CRM-style context sync here next.",
        planned: ["Calendar & meetings", "Spreadsheets / data room folders", "CRM or counterparty notes"],
      };
    case "student":
      return {
        title: "Specialized · Coursework",
        hint: "Semester-shaped context: syllabi, LMS, and project hubs so the twin matches how you actually study.",
        planned: ["LMS or Classroom", "Course Drive folders", "Group chat for cohorts"],
      };
    default:
      return {
        title: "Specialized · Your stack",
        hint: "Pick a discipline above so we know which third-party connector to prioritize. Onboarding will capture this officially later.",
        planned: ["Role-specific OAuth (one primary per persona)", "Calendar where it matters most", "The tools you use most often"],
      };
  }
}
