# Namespace: sample.blog

> Blog/CMS data model demonstrating inheritance, views,
> framework attributes, documentation, and behaviors.

---

## BaseModel ::interface

- id: identifier @pk @generated
- created_at: timestamp = now() @immutable
- updated_at: timestamp = now()

## Trackable ::interface

- version: integer = 1
- is_deleted: boolean = false
- deleted_at: timestamp?

---

## PostStatus ::enum

- draft: "Draft"
- review: "In Review"
- published: "Published"
- archived: "Archived"

## ContentFormat ::enum

- markdown: "Markdown"
- html: "HTML"
- plain: "Plain Text"

---

## User : BaseModel

> System user with authentication info.

- username: string(50) @unique @not_null @searchable
- email: email @unique @not_null
- password_hash: string(255) @not_null `[JsonIgnore]`
- display_name: string(100) @not_null
- bio: text?
- avatar_url: url?
- role: enum = "author"
  - admin: "Administrator"
  - editor: "Editor"
  - author: "Author"
  - subscriber: "Subscriber"
- is_active: boolean = true

# Computed
- full_profile_url: string @computed("'/users/' + username")

# Rollup
- post_count: integer @rollup(Post.author_id, count)
- published_post_count: integer @rollup(Post.author_id, count, where: "status = 'published'")

---

## Tag : BaseModel

- name: string(50) @unique @not_null @searchable
- slug: string(50) @unique @not_null
- color: string(7)? "Hex color code like #FF5733"

# Rollup
- usage_count: integer @rollup(PostTag.tag_id, count)

---

## Category : BaseModel

- name: string(100) @not_null @searchable
- slug: string(100) @unique @not_null
- description: text?
- parent_id: identifier? @reference(Category)?
- sort_order: integer = 0

# Lookup
- parent_name: string @lookup(parent_id.name)

---

## Post : BaseModel, Trackable

> Blog post with rich content and metadata.
> Supports multiple content formats and scheduled publishing.

- title: string(300) @not_null @searchable
- slug: string(300) @unique @not_null
- excerpt: string(500)? "Short summary for previews"
- content: text @not_null
- format: ContentFormat = "markdown"
- status: PostStatus = "draft"
- author_id: identifier @reference(User)!!
- category_id: identifier? @reference(Category)?
- featured_image: url?
- is_featured: boolean = false
- allow_comments: boolean = true
- published_at: timestamp?
- scheduled_at: timestamp?
- seo_title: string(70)? `[MaxLength(70)]` "SEO optimized title"
- seo_description: string(160)? `[MaxLength(160)]` "Meta description"
- reading_time_min: integer?
- view_count: long = 0 @min(0)

# Lookup
- author_name: string @lookup(author_id.display_name)
- author_avatar: url @lookup(author_id.avatar_url)
- category_name: string @lookup(category_id.name)

# Computed
- is_published: boolean @computed("status = 'published' AND published_at <= now()")
- word_count: integer @computed("LENGTH(content) / 5")

# Rollup
- comment_count: integer @rollup(Comment.post_id, count)
- avg_rating: decimal(3,2) @rollup(Comment.post_id, avg(rating))

### Indexes
- idx_author_status
  - fields: [author_id, status]
- idx_published
  - fields: [published_at]
  - where: "status = 'published'"

### Behaviors
- before_create: generate_slug_from_title
- before_update: increment_version
- after_publish: send_notifications

### Metadata
- table_name: posts
- cache_ttl: 300

---

## PostTag

> Many-to-many join between Post and Tag.

- post_id: identifier @reference(Post)
  - on_delete: cascade
- tag_id: identifier @reference(Tag)
  - on_delete: cascade

### PrimaryKey
- fields: [post_id, tag_id]

---

## Comment : BaseModel

- post_id: identifier @reference(Post)
- author_id: identifier? @reference(User)?
- parent_id: identifier? @reference(Comment)? # Threaded comments
- guest_name: string(100)? "Name for guest commenters"
- guest_email: email?
- body: text @not_null
- rating: integer? @min(1) @max(5) "Optional post rating"
- is_approved: boolean = false
- is_spam: boolean = false

# Lookup
- post_title: string @lookup(post_id.title)
- author_name: string @lookup(author_id.display_name)

# Rollup
- reply_count: integer @rollup(Comment.parent_id, count)

# Computed
- display_name: string @computed("COALESCE(author_name, guest_name, 'Anonymous')")

- @index(post_id, created_at)

---

## MediaAsset : BaseModel

- filename: string(255) @not_null
- original_name: string(255) @not_null
- mime_type: string(100) @not_null
- file_size: long @not_null "Size in bytes"
- width: integer? "Image width in pixels"
- height: integer? "Image height in pixels"
- alt_text: string(300)?
- uploaded_by: identifier @reference(User)!!
- storage_path: string(500) @not_null

# Computed
- file_size_mb: float @computed("file_size / 1048576.0")
- is_image: boolean @computed("mime_type LIKE 'image/%'")

---

## PublishedPosts ::view

> All published posts with author info.

### Source
- from: Post
- where: "status = 'published' AND is_deleted = false"
- order_by: "published_at desc"

---

## PopularPosts ::view @materialized

> Most viewed posts in the last 30 days.

### Source
- from: Post
- where: "status = 'published' AND published_at >= DATE_ADD(now(), -30, 'day')"
- order_by: "view_count desc"

### Refresh
- strategy: full
- interval: "6 hours"
