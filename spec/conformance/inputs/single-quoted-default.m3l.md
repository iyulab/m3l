## SingleQuotedDefault
- id: identifier @pk
- status: string(20) = 'PARSED' @not_null
- title: string(100) = 'two words'
- empty_note: string = ''
- plain: string = "double quoted"
- created_at: timestamp = `now()`
