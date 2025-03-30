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

> Stores information about book authors.

## Publisher : BaseModel
- name(Publisher Name): string(100) @not_null @unique
- address(Address): string(200)
- phone(Phone Number): string(20)
- email(Email): string(100) @validate(email)
- website(Website): string(200)?

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

> Stores book information.

## BookAuthor
- book_id(Book): identifier @fk(Book.id) @pk(1) @delete(cascade)
- author_id(Author): identifier @fk(Author.id) @pk(2) @delete(cascade)
- role(Role): string(50) = "author"

> Manages many-to-many relationships between books and authors.

## Category : BaseModel
- name(Category Name): string(100) @not_null @unique
- parent_id(Parent Category): identifier? @fk(Category.id)
- description(Description): text?

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

> Stores book reservation information.

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
- version: 1.0
- domain: "library"
- author: "System Designer"
- last_updated: "2023-11-15"
