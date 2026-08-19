import { render, screen } from '@testing-library/react';
import App from './App';

// Replaces the old CRA-boilerplate test ("renders learn react link") that
// asserted text neither crypton-demo nor crypton-admin ever contained and
// therefore always failed. This one asserts something the app actually does.
test('renders the login page when logged out', async () => {
  localStorage.clear();
  render(<App />);
  expect(await screen.findByRole('button', { name: /authenticate with passkey/i })).toBeInTheDocument();
});
