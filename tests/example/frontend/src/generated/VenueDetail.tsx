
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api";
import type { VenueRecord } from "./types";

export function VenueDetail() {
  const params = useParams();
  const navigate = useNavigate();
  const [row, setRow] = useState(null as VenueRecord | null);
  const [error, setError] = useState("");
  const id = Number(params.id);

  useEffect(() => {
    api.Venue.get(id).then(setRow).catch(function (err) {
      setError(String(err));
    });
  }, [id]);

  if (error.length > 0) {
    return <p className="error">{error}</p>;
  } else if (row === null) {
    return <p>Loading...</p>;
  } else {
    return (
      <section>
        <p><Link to="/venues">Back to list</Link></p>
        <h1>Venue {row.id}</h1>
        <dl>
          <dt>name</dt>
          <dd>{row.name}</dd>
          <dt>capacity</dt>
          <dd>{row.capacity}</dd>

        </dl>
        <p>
          <Link to={"/venues/" + row.id + "/edit"}>Edit</Link>
          {" "}
          <button type="button" onClick={function () {
            api.Venue.remove(row.id).then(function () {
              navigate("/venues");
            }).catch(function (err) {
              setError(String(err));
            });
          }}>Delete</button>
        </p>
      </section>
    );
  }
}
