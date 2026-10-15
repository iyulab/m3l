## TicketState ::enum
- open(Opened): "Waiting for someone to pick it up"
- in_progress(In progress): "Someone is working on it"
- closed: "Done"
- legacy(Legacy): "Carried over from the old system" @system

## Ticket
- id: identifier @pk
- priority: enum = "normal"
  - values:
    - normal(Normal): "Handled in order"
    - urgent(Urgent): "Handled first"
