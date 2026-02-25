# Library Management System

> Data model definition for a library management system.

## Timestampable
- created_at(Creation Date): timestamp = now()
- updated_at(Last Updated): timestamp = now() @on_update(now())

## BaseModel : Timestampable
- id(ID): identifier @pk @auto_increment

## Author : BaseModel
- name(Author Name): string(100) @not_null @idx
- bio(Biography): text?
- birth_date(Birth Date): date?
- nationality(Nationality): string(50)?
- photo_url(Photo): string(255)?

# Rollup
- book_count(Books Written): integer @rollup(BookAuthor.author_id, count)

> Stores information about book authors.

## Publisher : BaseModel
- name(Publisher Name): string(100) @not_null @unique
- address(Address): string(200)
- phone(Phone Number): string(20)
- email(Email): string(100) @validate(email)
- website(Website): string(200)?

# Rollup
- book_count(Published Books): integer @rollup(Book.publisher_id, count)
- active_book_count(Active Books): integer @rollup(Book.publisher_id, count, where: "status = 'available'")

> Stores information about publishers.

## Book : BaseModel
- title(Title): string(200) @not_null @idx
- isbn(ISBN): string(20) @not_null @unique
- publisher_id(Publisher): identifier @fk(Publisher.id)
- publication_date(Publication Date): date @not_null
- pages(Page Count): integer?
- language(Language): string(20) = "English"
- description(Description): text?
- cover_url(Cover Image): string(255)?
- quantity(Quantity): integer = 0
- location(Location): string(50)?
- status(Status): enum = "available"
  - available: "Available for loan"
  - borrowed: "Currently borrowed"
  - reserved: "Reserved"
  - archived: "Archived"
  - lost: "Lost"

# Lookup
- publisher_name(Publisher Name): string @lookup(publisher_id.name)

# Rollup
- active_loan_count(Active Loans): integer @rollup(Loan.book_id, count, where: "status = 'ongoing'")
- total_loan_count(Total Loans): integer @rollup(Loan.book_id, count)
- reservation_count(Reservations): integer @rollup(Reservation.book_id, count, where: "status = 'pending'")

# Computed
- is_available(Available): boolean @computed("status = 'available' AND quantity > 0")

> Stores book information.

## BookAuthor
- book_id(Book): identifier @fk(Book.id) @pk(1) @delete(cascade)
- author_id(Author): identifier @fk(Author.id) @pk(2) @delete(cascade)
- role(Role): string(50) = "author"

# Lookup
- book_title(Book Title): string @lookup(book_id.title)
- author_name(Author Name): string @lookup(author_id.name)

> Manages many-to-many relationships between books and authors.

## Category : BaseModel
- name(Category Name): string(100) @not_null @unique
- parent_id(Parent Category): identifier? @fk(Category.id)
- description(Description): text?

# Lookup
- parent_name(Parent Name): string? @lookup(parent_id.name)

# Rollup
- book_count(Books in Category): integer @rollup(BookCategory.category_id, count)

> Stores book category information.

## BookCategory
- book_id(Book): identifier @fk(Book.id) @pk(1) @delete(cascade)
- category_id(Category): identifier @fk(Category.id) @pk(2) @delete(cascade)

> Manages many-to-many relationships between books and categories.

## Member : BaseModel
- name(Name): string(100) @not_null
- email(Email): string(100) @not_null @unique @idx
- phone(Phone): string(20) @not_null
- address(Address): string(200)?
- membership_date(Membership Date): date = today()
- membership_type(Membership Type): enum = "standard"
  - standard: "Standard Member"
  - student: "Student Member"
  - senior: "Senior Member"
  - premium: "Premium Member"
- status(Status): enum = "active"
  - active: "Active"
  - suspended: "Suspended"
  - expired: "Expired"

# Rollup
- active_loan_count(Active Loans): integer @rollup(Loan.member_id, count, where: "status = 'ongoing'")
- total_loan_count(Total Loans): integer @rollup(Loan.member_id, count)
- total_fines(Total Fines): decimal(8,2) @rollup(Loan.member_id, sum(fine_amount))
- overdue_count(Overdue Loans): integer @rollup(Loan.member_id, count, where: "status = 'overdue'")
- last_loan_date(Last Loan): date? @rollup(Loan.member_id, max(loan_date))

# Computed from Rollup
- has_overdue(Has Overdue): boolean @computed("overdue_count > 0")
- can_borrow(Can Borrow): boolean @computed("status = 'active' AND active_loan_count < 5 AND overdue_count = 0")

> Stores library member information.

## Loan : BaseModel
- book_id(Book): identifier @fk(Book.id) @not_null
- member_id(Member): identifier @fk(Member.id) @not_null
- loan_date(Loan Date): date = today()
- due_date(Due Date): date @computed("loan_date + 14")
- return_date(Return Date): date?
- extended(Extended): boolean = false
- status(Status): enum = "ongoing"
  - ongoing: "Ongoing"
  - returned: "Returned"
  - overdue: "Overdue"
- fine_amount(Fine Amount): decimal(6,2) = 0
- notes(Notes): text?

# Lookup
- book_title(Book Title): string @lookup(book_id.title)
- book_isbn(ISBN): string @lookup(book_id.isbn)
- member_name(Member Name): string @lookup(member_id.name)
- member_email(Member Email): string @lookup(member_id.email)

# Computed
- is_overdue(Is Overdue): boolean @computed("status = 'ongoing' AND due_date < today()")
- days_overdue(Days Overdue): integer? @computed("CASE WHEN due_date < today() AND status = 'ongoing' THEN DATEDIFF(DAY, due_date, today()) END")

> Stores book loan information.

## Reservation : BaseModel
- book_id(Book): identifier @fk(Book.id) @not_null
- member_id(Member): identifier @fk(Member.id) @not_null
- reservation_date(Reservation Date): date = today()
- expiry_date(Expiry Date): date @computed("reservation_date + 7")
- status(Status): enum = "pending"
  - pending: "Pending"
  - fulfilled: "Fulfilled"
  - cancelled: "Cancelled"
  - expired: "Expired"

# Lookup
- book_title(Book Title): string @lookup(book_id.title)
- member_name(Member Name): string @lookup(member_id.name)

# Computed
- is_expired(Is Expired): boolean @computed("status = 'pending' AND expiry_date < today()")

> Stores book reservation information.

## OverdueLoans ::view
> Currently overdue loans for staff notifications

### Source
- from: Loan
- where: "status = 'ongoing' AND due_date < today()"
- order_by: due_date asc

- book_title: string @lookup(book_id.title)
- book_isbn: string @lookup(book_id.isbn)
- member_name: string @lookup(member_id.name)
- member_email: string @lookup(member_id.email)
- loan_date: date @from(Loan.loan_date)
- due_date: date @from(Loan.due_date)
- days_overdue: integer @computed("DATEDIFF(DAY, due_date, today())")

## MemberActivity ::view
> Member activity summary for librarian dashboard

### Source
- from: Member
- where: "status = 'active'"
- group_by: [Member.id, Member.name, Member.email, Member.membership_type]

- name: string @from(Member.name)
- email: string @from(Member.email)
- membership_type: string @from(Member.membership_type)
- active_loans: integer @rollup(Loan.member_id, count, where: "status = 'ongoing'")
- overdue_loans: integer @rollup(Loan.member_id, count, where: "status = 'overdue'")
- total_fines: decimal(8,2) @rollup(Loan.member_id, sum(fine_amount))
- last_loan: date? @rollup(Loan.member_id, max(loan_date))
- can_borrow: boolean @computed("active_loans < 5 AND overdue_loans = 0")

## PopularBooks ::view @materialized
> Most borrowed books - refreshed daily for reporting

### Source
- from: Book
- where: "status != 'lost'"
- order_by: total_loans desc

### Refresh
- strategy: scheduled
- interval: "daily 03:00"

- title: string @from(Book.title)
- isbn: string @from(Book.isbn)
- publisher_name: string? @lookup(Book.publisher_id.name)
- total_loans: integer @rollup(Loan.book_id, count)
- active_loans: integer @rollup(Loan.book_id, count, where: "status = 'ongoing'")
- pending_reservations: integer @rollup(Reservation.book_id, count, where: "status = 'pending'")

### Indexes
- book_search(Book Search)
  - fields: [title, isbn]
  - fulltext: true

- member_search(Member Search)
  - fields: [name, email, phone]

- active_loans(Active Loans)
  - fields: [member_id, status, due_date]

- book_loan_status(Book Loan Status)
  - fields: [book_id, status]

### Relations
- Book.id <-> Author.id {through: BookAuthor, as: authors, inverse_as: books}
- Book.id <-> Category.id {through: BookCategory, as: categories, inverse_as: books}
- Book.id <- Loan.book_id {type: o2m, as: loans}
- Book.id <- Reservation.book_id {type: o2m, as: reservations}
- Member.id <- Loan.member_id {type: o2m, as: loans}
- Member.id <- Reservation.member_id {type: o2m, as: reservations}
- Publisher.id <- Book.publisher_id {type: o2m, as: books}

### Metadata
- version: 1.1
- domain: "library"
- author: "System Designer"
- last_updated: "2026-02-25"
