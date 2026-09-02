use std::path::PathBuf;

use datuma_k::cli::run_project;
use datuma_k::project::check_project;

#[tokio::test]
async fn example_project_generates_backend_and_frontend() {
  let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/example");
  run_project(&root).await.expect("run");
  let models = std::fs::read_to_string(root.join("backend/generated/models.py")).expect("models");
  assert!(models.contains("Field(ge=1, le=500)"), "{models}");
  assert!(models.contains("Field(ge=1, le=10000)"), "{models}");
  assert!(models.contains("starts_at: datetime"), "{models}");
  let schemas =
    std::fs::read_to_string(root.join("frontend/src/generated/schemas.ts")).expect("schemas");
  assert!(schemas.contains(".min(1).max(500)"), "{schemas}");
  assert!(schemas.contains("z.string().min(1)"), "{schemas}");
  let transforms =
    std::fs::read_to_string(root.join("frontend/src/generated/transforms.ts")).expect("transforms");
  assert!(transforms.contains("toLocalDate"), "{transforms}");
  assert!(transforms.contains("parseEvent"), "{transforms}");
  let list =
    std::fs::read_to_string(root.join("frontend/src/generated/EventList.tsx")).expect("list");
  assert!(list.contains("EventList"), "{list}");
  let form =
    std::fs::read_to_string(root.join("frontend/src/generated/EventForm.tsx")).expect("form");
  assert!(form.contains("EventForm"), "{form}");
  assert!(form.contains("datetime-local"), "{form}");
  assert!(!form.contains("@dk"), "{form}");
  assert!(!models.contains("@dk"), "{models}");
  assert!(root.join("frontend/src/generated/VenueDetail.tsx").exists());
  assert!(root.join("frontend/src/generated/VenueForm.tsx").exists());
  let check = check_project(&root).await.expect("check");
  assert!(check.ok, "{:?}", check.diagnostics);
  run_project(&root).await.expect("rerun");
  assert!(root.join("backend/generated/models.py").exists());
  assert!(root.join("frontend/src/generated/EventList.tsx").exists());
}
