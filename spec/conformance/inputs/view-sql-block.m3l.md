## User
- id: identifier @pk
- name: string(100)
- department: string(50)
- is_active: boolean = true

## UserReport ::view
### Source
```sql
FROM User u
WHERE u.department = 'Engineering'
ORDER BY u.name ASC
```

- user_name: string @from(User.name)
- dept: string @from(User.department)
