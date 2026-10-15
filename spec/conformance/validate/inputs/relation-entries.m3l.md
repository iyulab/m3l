# Namespace: conformance.relations

## Person
- id: identifier @pk @generated
- name: string(50)

## Post
- id: identifier @pk @generated
- author_id: identifier @reference(Person)
- owner: identifier @reference(Person)
- title: string(100)

### Relations
- >writer
  - target: Person
  - from: author_id
- @relation(owned_by, -> Person, from: owner)
- <comments
  - target: Comment.post_id
- >ghost
  - target: Person
  - from: nope_id
- >titled
  - from: title
- >misdirected
  - target: Comment
  - from: author_id
- @relation(strays, <- Comment.note)

## Comment
- id: identifier @pk @generated
- post_id: identifier @reference(Post)
- note: string(200)
