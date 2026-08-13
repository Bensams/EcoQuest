import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { ErrorNote } from '../../components/ui/EmptyState';
import { Field, Select, TextArea, TextInput } from '../../components/ui/Field';
import { PageHeader } from '../../components/ui/PageHeader';
import { ApiError, post } from '../../lib/api';

const orgTypes = ['NGO', 'Community group', 'School', 'Business', 'Government', 'Other'].map((type) => ({
  value: type,
  label: type,
}));

export function CreateOrganizationPage() {
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [organizationType, setOrganizationType] = useState('NGO');
  const [location, setLocation] = useState('');
  const [description, setDescription] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      await post('/api/organizations', {
        name,
        organization_type: organizationType,
        location,
        description,
      });
      navigate('/org', { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Could not submit the application. Try again.');
    } finally {
      setPending(false);
    }
  };

  return (
    <div className="mx-auto max-w-xl">
      <PageHeader
        eyebrow="Organization"
        title="Create an organization"
        detail="Your application will be reviewed by an EcoQuest administrator. Once approved, you become the owner and can run events."
      />
      <Card>
        <form onSubmit={submit} className="grid gap-4">
          <Field label="Organization name" hint="Use the legal or commonly used name.">
            <TextInput
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={120}
              autoComplete="organization"
              required
            />
          </Field>
          <Field label="Organization type">
            <Select
              value={organizationType}
              onChange={setOrganizationType}
              options={orgTypes}
              aria-label="Organization type"
            />
          </Field>
          <Field label="Location" hint="Where does this organization primarily operate?">
            <TextInput value={location} onChange={(e) => setLocation(e.target.value)} required />
          </Field>
          <Field label="Description" hint="What does your organization do?">
            <TextArea value={description} onChange={(e) => setDescription(e.target.value)} maxLength={2000} />
          </Field>
          {error && <ErrorNote message={error} />}
          <Button type="submit" disabled={pending}>
            {pending ? 'Submitting…' : 'Submit application'}
          </Button>
        </form>
      </Card>
    </div>
  );
}