
import { useEffect, useState, type FormEvent } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api";
import { EventSchema } from "./schemas";
import { toDatetimeLocalInput, toLocalDate } from "./transforms";

import type { Event } from "./types";

type FormState = {
  title: string;
  capacity: string;
  starts_at: string;

};

const empty: FormState = {
  title: "",
  capacity: "",
  starts_at: "",

};

export function EventForm() {
  const params = useParams();
  const navigate = useNavigate();
  const editing = params.id !== undefined;
  const [form, setForm] = useState(empty);
  const [error, setError] = useState("");
  const id = Number(params.id);

  useEffect(() => {
    if (editing) {
      api.Event.get(id).then(function (row) {
        setForm({
          title: String(row.title),
          capacity: String(row.capacity),
          starts_at: toDatetimeLocalInput(row.starts_at),

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
    const checked = EventSchema.safeParse({
      title: form.title,
      capacity: Number(form.capacity),
      starts_at: form.starts_at,

    });
    if (!checked.success) {
      setError(checked.error.issues.map(function (issue) { return issue.message; }).join("; "));
    } else {
      const payload: Event = {
        title: form.title,
        capacity: Number(form.capacity),
        starts_at: toLocalDate(form.starts_at),

      };
      const saved = editing ? api.Event.update(id, payload) : api.Event.create(payload);
      saved.then(function (row) {
        navigate("/events/" + row.id);
      }).catch(function (err) {
        setError(String(err));
      });
    }
  }

  return (
    <section>
      <p><Link to="/events">Back to list</Link></p>
      <h1>{editing ? "Update" : "Create"} Event</h1>
      {error.length > 0 ? <p className="error">{error}</p> : null}
      <form onSubmit={onSubmit}>
        <label>
          title
          <input type="text" value={form.title} onChange={function (ev) { setField("title", ev.target.value); }} />
        </label>
        <label>
          capacity
          <input type="number" value={form.capacity} onChange={function (ev) { setField("capacity", ev.target.value); }} />
        </label>
        <label>
          starts_at
          <input type="datetime-local" value={form.starts_at} onChange={function (ev) { setField("starts_at", ev.target.value); }} />
        </label>

        <button type="submit">Save</button>
      </form>
    </section>
  );
}
