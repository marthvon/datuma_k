
import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api";
import { VenueSchema } from "./schemas";

import type { Venue } from "./types";

type FormState = {
  name: string;
  capacity: string;

};

const empty: FormState = {
  name: "",
  capacity: "",

};

export function VenueForm() {
  const params = useParams();
  const navigate = useNavigate();
  const editing = params.id !== undefined;
  const [form, setForm] = useState(empty);
  const [error, setError] = useState("");
  const id = Number(params.id);

  useEffect(() => {
    if (editing) {
      api.Venue.get(id).then(function (row) {
        setForm({
          name: String(row.name),
          capacity: String(row.capacity),

        });
      }).catch(function (err) {
        setError(String(err));
      });
    }
  }, [editing, id]);

  function setField(name: keyof FormState, value: string) {
    setForm(function (prev) {
      return { ...prev, [name]: value };
    });
  }

  function onSubmit(ev: FormEvent) {
    ev.preventDefault();
    const checked = VenueSchema.safeParse({
      name: form.name,
      capacity: Number(form.capacity),

    });
    if (!checked.success) {
      setError(checked.error.issues.map(function (issue) { return issue.message; }).join("; "));
    } else {
      const payload: Venue = {
        name: form.name,
        capacity: Number(form.capacity),

      };
      const saved = editing ? api.Venue.update(id, payload) : api.Venue.create(payload);
      saved.then(function (row) {
        navigate("/venues/" + row.id);
      }).catch(function (err) {
        setError(String(err));
      });
    }
  }

  return (
    <section>
      <p><Link to="/venues">Back to list</Link></p>
      <h1>{editing ? "Update" : "Create"} Venue</h1>
      {error.length > 0 ? <p className="error">{error}</p> : null}
      <form onSubmit={onSubmit}>
        <label>
          name
          <input type="text" value={form.name} onChange={function (ev) { setField("name", ev.target.value); }} />
        </label>
        <label>
          capacity
          <input type="number" value={form.capacity} onChange={function (ev) { setField("capacity", ev.target.value); }} />
        </label>

        <button type="submit">Save</button>
      </form>
    </section>
  );
}
