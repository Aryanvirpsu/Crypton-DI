import { CryptonTransport } from '../client';

export class ContactModule {
  constructor(private client: CryptonTransport) {}

  submit(data: { name: string, email: string, message: string, company?: string }): Promise<any> {
    return this.client.post("/api/contact", data);
  }
}
