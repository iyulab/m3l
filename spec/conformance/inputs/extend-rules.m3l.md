## Asset
- id: identifier @pk
- name: string(100)

## Asset ::extend
- name: string(50)
- code: string(20) @pk

## Ghost ::extend
- note: string?

## Asset ::extend : Tracked
- owner: string?

## Asset ::extend
- tag: string?

### Indexes
- idx_tag: [tag]

## Asset ::extend()
- label: string?
