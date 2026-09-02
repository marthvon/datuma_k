
import { z } from "zod";


export const EventSchema = z.object({
  title: z.string().min(1),
  capacity: z.number().min(1).max(500),
  starts_at: z.string(),

});


export const VenueSchema = z.object({
  name: z.string().min(1),
  capacity: z.number().min(1).max(10000),

});


