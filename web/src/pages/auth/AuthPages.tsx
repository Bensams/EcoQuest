import { useState, type FormEvent } from 'react';
import { Leaf } from 'lucide-react';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../lib/auth';
import { ApiError } from '../../lib/api';
import { Button } from '../../components/ui/Button';
import { Card } from '../../components/ui/Card';
import { ErrorNote } from '../../components/ui/EmptyState';
import { Field, TextInput } from '../../components/ui/Field';

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const handleSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      await login(email, password);
      navigate('/app/missions', { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Sign in failed. Try again.');
    } finally {
      setPending(false);
    }
  };

  return (
    <AuthShell title="Sign in to EcoQuest" subtitle="Welcome back. Pick up where your missions left off.">
      <form onSubmit={handleSubmit} className="grid gap-4">
        <Field label="Email">
          <TextInput type="email" value={email} onChange={(e) => setEmail(e.target.value)} autoComplete="email" required />
        </Field>
        <Field label="Password">
          <TextInput
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="current-password"
            required
          />
        </Field>
        {error && <ErrorNote message={error} />}
        <Button type="submit" disabled={pending}>
          {pending ? 'Signing in…' : 'Sign in'}
        </Button>
      </form>
      <p className="mt-6 text-center text-sm text-forest-muted">
        New here?{' '}
        <Link to="/register" className="font-medium text-leaf hover:text-leaf-dark">
          Create an account
        </Link>
      </p>
    </AuthShell>
  );
}

export function RegisterPage() {
  const { register } = useAuth();
  const navigate = useNavigate();
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [role, setRole] = useState<'PLAYER' | 'ORGANIZATION_MEMBER'>('PLAYER');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setError(null);
    setPending(true);
    try {
      await register(username, email, password, role);
      navigate('/app/missions', { replace: true });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : 'Account creation failed. Try again.');
    } finally {
      setPending(false);
    }
  };

  return (
    <AuthShell title="Create an account" subtitle="Join local environmental action in a few steps.">
      <form onSubmit={submit} className="grid gap-4">
        <Field label="Username">
          <TextInput value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" required />
        </Field>
        <Field label="Email">
          <TextInput type="email" value={email} onChange={(e) => setEmail(e.target.value)} autoComplete="email" required />
        </Field>
        <Field label="Password">
          <TextInput
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="new-password"
            required
            minLength={12}
          />
        </Field>
        <Field label="I am joining as">
          <div className="grid grid-cols-2 gap-2">
            {(['PLAYER', 'ORGANIZATION_MEMBER'] as const).map((option) => (
              <label
                key={option}
                className={`flex cursor-pointer items-center justify-center rounded-lg border px-3 py-2 text-sm font-medium transition-colors ${
                  role === option ? 'border-leaf bg-sage-soft text-leaf' : 'border-sage text-forest-muted hover:bg-sage-soft'
                }`}
              >
                <input
                  type="radio"
                  name="role"
                  className="sr-only"
                  checked={role === option}
                  onChange={() => setRole(option)}
                />
                {option === 'PLAYER' ? 'Volunteer' : 'Organization member'}
              </label>
            ))}
          </div>
          <span className="text-xs text-forest-muted">Admin accounts are granted by the platform, not self-assigned.</span>
        </Field>
        {error && <ErrorNote message={error} />}
        <Button type="submit" disabled={pending}>
          {pending ? 'Creating account…' : 'Create account'}
        </Button>
      </form>
      <p className="mt-6 text-center text-sm text-forest-muted">
        Already registered?{' '}
        <Link to="/login" className="font-medium text-leaf hover:text-leaf-dark">
          Sign in
        </Link>
      </p>
    </AuthShell>
  );
}

function AuthShell({ title, subtitle, children }: { title: string; subtitle: string; children: React.ReactNode }) {
  return (
    <div className="grid min-h-screen place-items-center bg-canvas px-4">
      <Card className="w-full max-w-sm">
        <div className="mb-6 flex flex-col items-center gap-2 text-center">
          <span className="grid size-10 place-items-center rounded-xl bg-leaf text-white">
            <Leaf className="size-5" aria-hidden="true" />
          </span>
          <h1 className="text-xl font-semibold text-forest">{title}</h1>
          <p className="text-sm text-forest-muted">{subtitle}</p>
        </div>
        {children}
      </Card>
    </div>
  );
}