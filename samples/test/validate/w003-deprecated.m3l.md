# Namespace: test.w003

## OldModel

- id: identifier @pk
- old_timestamp: datetime
- ref_id: identifier @reference(OldModel) @cascade
