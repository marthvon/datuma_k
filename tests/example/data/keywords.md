| keyword | kind | description | purpose | platforms |
| --- | --- | --- | --- | --- |
| Event | model | A scheduled gathering with a title, capacity, and start time. | Shared booking record generated into API models and web list/detail/form pages. | api_server, web_frontend |
| Venue | model | A physical location that can host events. | Shared venue record generated into API models and web list/detail/form pages. | api_server, web_frontend |
| Resource | trait | Marks models that are CRUD resources. | Selects which models ngin emits as Pydantic models, Zod schemas, and React pages. | api_server, web_frontend |
| title | field | Human-readable name of an event. | Required text shown on event forms and detail views. | api_server, web_frontend |
| name | field | Human-readable name of a venue. | Required text shown on venue forms and detail views. | api_server, web_frontend |
| capacity | field | Maximum number of people the event or venue holds. | Numeric limit enforced as Pydantic Field ge/le and Zod min/max. | api_server, web_frontend |
| starts_at | field | When the event begins. | Datetime stored on the API and edited as a local datetime-local input on the web. | api_server, web_frontend |
| text_type | type | UTF-8 text. | Maps to str / string in generated Python and TypeScript. | api_server, web_frontend |
| int_type | type | Integer number. | Maps to int / number with numeric min/max constraints. | api_server, web_frontend |
| datetime_type | type | Date and time. | Maps to datetime / Date and datetime-local inputs when marked local. | api_server, web_frontend |
| required | attribute | Value must be present and non-empty. | Zod .min(1) on strings; required form fields. | api_server, web_frontend |
| min | attribute | Inclusive lower bound. | Pydantic Field(ge=) and Zod .min() on numeric fields. | api_server, web_frontend |
| max | attribute | Inclusive upper bound. | Pydantic Field(le=) and Zod .max() on numeric fields. | api_server, web_frontend |
| local | attribute | Datetime is interpreted in the user's local timezone. | Web datetime-local inputs and local/UTC transform helpers. | web_frontend |
