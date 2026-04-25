export interface BgpPeer {
  asn: number;
  name: string | null;
  country: string | null;
}

export interface BgpInfo {
  asn: number | null;
  as_name: string | null;
  as_country: string | null;
  prefixes_v4: string[];
  prefixes_v6: string[];
  peers: BgpPeer[];
}

export interface DnsRecord {
  record_type: string;
  value: string;
  ttl: number;
  dnssec_validated: boolean;
}

export interface IpRecord {
  id: string;
  order: number;
  ip: string;
  country: string | null;
  owner_name: string | null;
  address: string | null;
  emails: string[];
  abuse_emails: string[];
  phone: string | null;
  fax: string | null;
  from_ip: string | null;
  to_ip: string | null;
  status: string | null;
  whois_source: string | null;
  network_name: string | null;
  contact_name: string | null;
  allocated: string | null;
  host_name: string | null;
  resolved_name: string | null;
  cidr: string | null;
  postal_code: string | null;
  abuse_contact: string | null;
  raw_whois: string | null;
  raw_rdap: unknown;
  geo_lat:     number | null;
  geo_lon:     number | null;
  geo_city:    string | null;
  geo_country: string | null;
  dns_records: DnsRecord[];
  lookup_errors: string[];
  bgp: BgpInfo | null;
}

export interface ColumnDef {
  key: string;
  label: string;
  minWidth: number;
  getValue: (row: IpRecord) => string;
}

export interface SortState {
  key: string;
  dir: 'asc' | 'desc';
}

export const ALL_COLUMNS: ColumnDef[] = [
  { key: 'order',        label: '#',             minWidth: 52,  getValue: r => String(r.order) },
  { key: 'ip',           label: 'IP Address',    minWidth: 140, getValue: r => r.ip },
  { key: 'country',      label: 'Country',       minWidth: 100, getValue: r => r.country ?? '' },
  { key: 'owner_name',   label: 'Owner',         minWidth: 160, getValue: r => r.owner_name ?? '' },
  { key: 'network_name', label: 'Network',       minWidth: 140, getValue: r => r.network_name ?? '' },
  { key: 'cidr',         label: 'CIDR',          minWidth: 130, getValue: r => r.cidr ?? '' },
  { key: 'from_ip',      label: 'From IP',       minWidth: 130, getValue: r => r.from_ip ?? '' },
  { key: 'to_ip',        label: 'To IP',         minWidth: 130, getValue: r => r.to_ip ?? '' },
  { key: 'status',       label: 'Status',        minWidth: 100, getValue: r => r.status ?? '' },
  { key: 'address',      label: 'Address',       minWidth: 200, getValue: r => r.address ?? '' },
  { key: 'emails',       label: 'Email',         minWidth: 180, getValue: r => r.emails.join('; ') },
  { key: 'abuse_emails', label: 'Abuse Email',   minWidth: 180, getValue: r => r.abuse_emails.join('; ') },
  { key: 'phone',        label: 'Phone',         minWidth: 120, getValue: r => r.phone ?? '' },
  { key: 'fax',          label: 'Fax',           minWidth: 110, getValue: r => r.fax ?? '' },
  { key: 'whois_source', label: 'WHOIS Source',  minWidth: 130, getValue: r => r.whois_source ?? '' },
  { key: 'contact_name', label: 'Contact',       minWidth: 150, getValue: r => r.contact_name ?? '' },
  { key: 'allocated',    label: 'Allocated',     minWidth: 110, getValue: r => r.allocated ?? '' },
  { key: 'host_name',    label: 'Hostname',      minWidth: 180, getValue: r => r.host_name ?? '' },
  { key: 'resolved_name',label: 'Resolved',      minWidth: 180, getValue: r => r.resolved_name ?? '' },
  { key: 'postal_code',  label: 'Postal Code',   minWidth: 110, getValue: r => r.postal_code ?? '' },
  { key: 'abuse_contact',label: 'Abuse Contact', minWidth: 160, getValue: r => r.abuse_contact ?? '' },
];

export const DEFAULT_VISIBLE_COLUMNS = ALL_COLUMNS.map(c => c.key);
