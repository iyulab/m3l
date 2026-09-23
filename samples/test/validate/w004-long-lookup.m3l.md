# Namespace: test.w004

## Region

- id: identifier @pk
- name: string

---

## Country

- id: identifier @pk
- region_id: identifier @reference(Region)

---

## City

- id: identifier @pk
- country_id: identifier @reference(Country)

---

## Site

- id: identifier @pk
- city_id: identifier @reference(City)

---

## Source

- id: identifier @pk
- site_id: identifier @reference(Site)

### Lookup
- deep_val: string @lookup(site_id.city_id.country_id.region_id.name)
